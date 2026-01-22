use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};

#[derive(Debug, Deserialize)]
struct Domain {
    name: String,
    url: String,
    #[serde(default = "default_timeout")]
    timeout_seconds: u64,
}

fn default_timeout() -> u64 {
    10
}

#[derive(Debug, Deserialize)]
struct DomainsConfig {
    domains: Vec<Domain>,
}

#[derive(Debug, Serialize)]
struct CheckResult {
    name: String,
    url: String,
    success: bool,
    error: Option<String>,
    status_code: Option<u16>,
    response_time_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct SlackMessage {
    text: String,
    blocks: Vec<SlackBlock>,
}

#[derive(Debug, Serialize)]
struct SlackBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<SlackText>,
    fields: Option<Vec<SlackText>>,
}

#[derive(Debug, Serialize)]
struct SlackText {
    #[serde(rename = "type")]
    text_type: String,
    text: String,
}

async fn check_domain(client: &Client, domain: &Domain) -> CheckResult {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(domain.timeout_seconds);

    match client
        .get(&domain.url)
        .timeout(timeout)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            let response_time = start.elapsed().as_millis() as u64;
            
            CheckResult {
                name: domain.name.clone(),
                url: domain.url.clone(),
                success: status.is_success(),
                error: if status.is_success() {
                    None
                } else {
                    Some(format!("HTTP {}", status.as_u16()))
                },
                status_code: Some(status.as_u16()),
                response_time_ms: Some(response_time),
            }
        }
        Err(e) => {
            let response_time = start.elapsed().as_millis() as u64;
            let error_msg = if e.is_timeout() {
                "Timeout".to_string()
            } else if e.is_connect() {
                "Connection failed".to_string()
            } else if e.is_request() {
                "Request failed".to_string()
            } else {
                format!("Error: {}", e)
            };

            CheckResult {
                name: domain.name.clone(),
                url: domain.url.clone(),
                success: false,
                error: Some(error_msg),
                status_code: None,
                response_time_ms: Some(response_time),
            }
        }
    }
}

async fn send_slack_notification(webhook_url: &str, failures: &[CheckResult]) -> Result<(), Error> {
    if failures.is_empty() {
        return Ok(());
    }

    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let failure_count = failures.len();
    
    let header_text = format!(
        "🚨 *Uptime Alert: {} domain{} down*",
        failure_count,
        if failure_count == 1 { " is" } else { "s are" }
    );

    let mut blocks = vec![
        SlackBlock {
            block_type: "header".to_string(),
            text: Some(SlackText {
                text_type: "plain_text".to_string(),
                text: format!("Uptime Alert: {} domain{} down", failure_count, if failure_count == 1 { " is" } else { "s are" }),
            }),
            fields: None,
        },
        SlackBlock {
            block_type: "section".to_string(),
            text: Some(SlackText {
                text_type: "mrkdwn".to_string(),
                text: format!("*Check Time:* {}", timestamp),
            }),
            fields: None,
        },
        SlackBlock {
            block_type: "divider".to_string(),
            text: None,
            fields: None,
        },
    ];

    for failure in failures {
        let error_text = failure.error.as_ref().unwrap_or(&"Unknown error".to_string());
        
        blocks.push(SlackBlock {
            block_type: "section".to_string(),
            text: None,
            fields: Some(vec![
                SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: format!("*Domain:*\n{}", failure.name),
                },
                SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: format!("*Error:*\n{}", error_text),
                },
                SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: format!("*URL:*\n<{}|{}>", failure.url, failure.url),
                },
                SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: format!("*Response Time:*\n{}ms", failure.response_time_ms.unwrap_or(0)),
                },
            ]),
        });
    }

    let message = SlackMessage {
        text: header_text,
        blocks,
    };

    let client = Client::new();
    client
        .post(webhook_url)
        .json(&message)
        .send()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(_req: Request) -> Result<Response<Body>, Error> {
    // Load domains configuration
    let config_content = include_str!("../domains.json");
    let config: DomainsConfig = serde_json::from_str(config_content)
        .map_err(|e| Error::from(format!("Failed to parse domains.json: {}", e)))?;

    // Create HTTP client
    let client = Client::builder()
        .user_agent("MagnetWatchtower/1.0")
        .build()
        .map_err(|e| Error::from(e.to_string()))?;

    // Check all domains concurrently
    let mut tasks = Vec::new();
    for domain in &config.domains {
        let client = client.clone();
        let domain = domain.clone();
        tasks.push(tokio::spawn(async move {
            check_domain(&client, &domain).await
        }));
    }

    // Collect results
    let mut results = Vec::new();
    for task in tasks {
        match task.await {
            Ok(result) => results.push(result),
            Err(e) => eprintln!("Task failed: {}", e),
        }
    }

    // Filter failures
    let failures: Vec<CheckResult> = results
        .iter()
        .filter(|r| !r.success)
        .cloned()
        .collect();

    // Send Slack notification if there are failures
    if !failures.is_empty() {
        if let Ok(webhook_url) = std::env::var("SLACK_WEBHOOK_URL") {
            if let Err(e) = send_slack_notification(&webhook_url, &failures).await {
                eprintln!("Failed to send Slack notification: {}", e);
            }
        } else {
            eprintln!("SLACK_WEBHOOK_URL not set, skipping notification");
        }
    }

    // Return summary response
    let summary = serde_json::json!({
        "timestamp": Utc::now().to_rfc3339(),
        "total_checked": results.len(),
        "successful": results.iter().filter(|r| r.success).count(),
        "failed": failures.len(),
        "results": results,
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&summary)?))?)
}
