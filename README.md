# ghfetch

A GitHub stats API built with Rust on Cloudflare Workers. Returns aggregated repository and contribution data for any GitHub user via a single HTTP request.

## Features

- Aggregates owned, collaborated, and contributed repositories
- Per-IP and global rate limiting backed by Cloudflare KV
- Response caching with 5-minute TTL to protect GitHub token limits
- Runs at the edge with sub-10ms cached response times

## Live Instance

```
https://ghfetch.carlosranara.workers.dev
```

```bash
curl "https://ghfetch.carlosranara.workers.dev/?username=torvalds"
```

## Response Shape

```json
{
  "ok": true,
  "data": {
    "displayName": "string",
    "avatarUrl": "string",
    "bio": "string",
    "accountCreatedAt": "ISO 8601",
    "totalRepos": 0,
    "totalStars": 0,
    "totalContributions": 0,
    "totalCommits": 0,
    "totalPrs": 0,
    "totalIssues": 0,
    "followers": 0,
    "following": 0,
    "mostStarredRepo": {
      "name": "string",
      "stars": 0,
      "url": "string"
    },
    "languages": [
      {
        "name": "string",
        "percentage": 0,
        "color": "string"
      }
    ]
  }
}
```

On error:
```json
{
  "ok": false,
  "error": "string"
}
```

## Rate Limiting

| Limit | Value |
|---|---|
| Per IP global | 100 req/min |
| Per IP per username | 30 req/min |
| Block duration | 5 min |

Exceeding limits returns `429 Too Many Requests`.

## Deploy Your Own

### Prerequisites

- [Rust](https://rustup.rs)
- [wrangler](https://developers.cloudflare.com/workers/wrangler/install-and-update/)
- A Cloudflare account
- A GitHub personal access token with `read:user` and `repo` scopes

### Setup

1. Clone the repo

```bash
git clone https://github.com/AmaneKai/ghfetch
cd ghfetch
```

2. Create a KV namespace

```bash
wrangler kv namespace create RATE_LIMIT_KV
```

Copy the `id` from the output and update `wrangler.toml`:

```toml
[[kv_namespaces]]
binding = "RATE_LIMIT_KV"
id = "your-kv-namespace-id"
```

3. Set your GitHub token

```bash
wrangler secret put GITHUB_TOKEN
```

4. Deploy

```bash
wrangler deploy
```

5. Test

```bash
curl "https://<your-worker>.workers.dev/?username=<github-username>"
```

## Development

```bash
# Run tests
cargo test

# Local dev server (uses remote KV bindings)
wrangler dev --remote
```

## Stack

- Rust
- Cloudflare Workers (`worker-rs`)
- Cloudflare KV
- GitHub GraphQL API v4

## License

MIT
