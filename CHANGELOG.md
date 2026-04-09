# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [1.0.0] - 2026-04-09

### Added
- Initial release
- `GET /v1/stats?username=<github-username>` — aggregated GitHub stats endpoint
- `GET /health` — health check endpoint
- `GET /` — API info and endpoint documentation
- KV response caching with 5-minute TTL
- Per-IP rate limiting (10 req/min, 5-minute block)
- Global IP rate limiting (100 req/min)
- Global username rate limiting (60 unique usernames/min) to prevent enumeration attacks
- Rate limit response headers (`X-RateLimit-Remaining`, `X-RateLimit-Reset`, `X-Cache`)
- Security headers (`X-Content-Type-Options`, `X-Frame-Options`, `X-XSS-Protection`, `Referrer-Policy`)
- CORS support
- Username validation (GitHub username format enforcement)
- Aggregates owned, collaborated, and contributed repositories via GitHub GraphQL API v4
- 20 unit tests covering validation and repository processing logic
