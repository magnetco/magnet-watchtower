# Magnet Watchtower

A lightweight, stateless uptime monitoring service built with Rust that runs on Vercel serverless functions. Monitors domain uptime hourly and sends Slack notifications when sites go down.

## Features

- 🦀 **Built with Rust** - Fast, efficient, and minimal memory footprint
- ⚡ **Serverless** - Runs on Vercel with zero infrastructure management
- 🔔 **Slack Notifications** - Instant alerts when domains are unreachable
- 💰 **Cost-Effective** - Stateless design, stays within free tier limits
- ⏰ **Hourly Checks** - Automated monitoring via Vercel Cron
- 🚀 **Concurrent Checks** - All domains checked in parallel for speed

## Architecture

The service runs as a single Vercel serverless function that:
1. Loads domain list from `domains.json`
2. Checks all domains concurrently with configurable timeouts
3. Sends Slack notifications for any failures
4. Returns a JSON summary of all checks

## Setup

### Prerequisites

- [Rust](https://rustup.rs/) (for local development)
- [Vercel CLI](https://vercel.com/download) (optional, for local testing)
- A Slack workspace with webhook access

### 1. Clone and Install

```bash
git clone <your-repo-url>
cd magnet-watchtower
cargo build --release
```

### 2. Configure Domains

Edit `domains.json` to add your domains:

```json
{
  "domains": [
    {
      "name": "Client A Website",
      "url": "https://clienta.com",
      "timeout_seconds": 10
    },
    {
      "name": "Client B API",
      "url": "https://api.clientb.com/health",
      "timeout_seconds": 15
    }
  ]
}
```

**Configuration Options:**
- `name` - Friendly name for the domain (appears in notifications)
- `url` - Full URL to check (including protocol)
- `timeout_seconds` - Request timeout in seconds (default: 10)

### 3. Set Up Slack Webhook

1. Go to your Slack workspace settings
2. Navigate to **Apps** → **Incoming Webhooks**
3. Create a new webhook for your desired channel
4. Copy the webhook URL

### 4. Configure Environment Variables

Create a `.env.local` file for local testing:

```bash
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/YOUR/WEBHOOK/URL
```

For Vercel deployment, set the environment variable:

```bash
vercel env add SLACK_WEBHOOK_URL
```

Or via the Vercel dashboard: **Project Settings** → **Environment Variables**

### 5. Deploy to Vercel

```bash
vercel deploy --prod
```

The cron job will automatically run every hour at the top of the hour.

## Local Testing

### Test the Function Locally

```bash
# Using Vercel CLI
vercel dev

# Then visit http://localhost:3000/api/check
```

### Manual Rust Testing

You can also test the core logic directly with Cargo (requires minor modifications to bypass Vercel runtime).

## How It Works

### Success Criteria

A domain is considered "up" if:
- HTTP request completes within the timeout period
- Response status code is 200-299

### Failure Detection

A domain is marked as "down" if:
- Request times out
- Connection fails (DNS, network issues)
- HTTP status code is not 200-299

### Notifications

Slack notifications are sent **only when failures occur**. Each notification includes:
- Number of domains down
- Timestamp of the check
- For each failed domain:
  - Domain name
  - URL
  - Error type (timeout, HTTP error, connection failure)
  - Response time

### Response Format

The function returns a JSON summary:

```json
{
  "timestamp": "2026-01-21T12:00:00Z",
  "total_checked": 5,
  "successful": 4,
  "failed": 1,
  "results": [
    {
      "name": "Client A Website",
      "url": "https://clienta.com",
      "success": true,
      "error": null,
      "status_code": 200,
      "response_time_ms": 245
    }
  ]
}
```

## Cost Optimization

This service is designed to stay within Vercel's free tier:

- **Hourly checks**: 720 invocations/month (well under 100GB-hours limit)
- **Fast execution**: Rust's efficiency means minimal compute time
- **No database**: Stateless design eliminates storage costs
- **Concurrent requests**: Parallel checking reduces total execution time

## Monitoring the Monitor

To ensure the monitoring service itself is working:

1. Check Vercel function logs in the dashboard
2. Set up a test domain that you can intentionally break
3. Monitor your Slack channel for notifications
4. Review the cron job execution history in Vercel

## Troubleshooting

### No notifications received

- Verify `SLACK_WEBHOOK_URL` is set correctly in Vercel
- Check Vercel function logs for errors
- Test the webhook URL manually with curl

### Timeouts occurring

- Increase `timeout_seconds` for slow domains
- Check if the domain has rate limiting
- Verify network connectivity from Vercel's infrastructure

### Cron not running

- Verify `vercel.json` cron configuration
- Check that you're on a Vercel plan that supports cron
- Review cron execution logs in Vercel dashboard

## Project Structure

```
magnet-watchtower/
├── api/
│   └── check.rs          # Main serverless function
├── domains.json          # Domain configuration
├── Cargo.toml           # Rust dependencies
├── vercel.json          # Vercel config & cron schedule
├── .gitignore           # Git ignore rules
├── .env.example         # Environment variable template
└── README.md            # This file
```

## Adding/Removing Domains

Simply edit `domains.json` and redeploy:

```bash
# Edit domains.json
vim domains.json

# Deploy changes
vercel deploy --prod
```

Changes take effect immediately on the next cron run.

## License

MIT

## Support

For issues or questions, please open an issue in the repository.
