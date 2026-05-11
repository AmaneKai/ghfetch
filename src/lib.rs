use worker::{event, Env, Request, Response, Result};

mod types;
mod rate_limit;
mod github;
mod processor;
mod response;
mod tests;

use types::{ClientId, RateConfig, Username};
use rate_limit::Limiter;
use github::GitHubClient;
use processor::{build_stats, process_repos};
use response::{cors_preflight, err, health, success};
use shared::github::GitHubStats;

const CACHE_TTL: u64 = 300; // 5 minutes
const VERSION: &str = "1.0.0";

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: worker::Context) -> Result<Response> {
    if req.method() == worker::Method::Options {
        return cors_preflight();
    }

    let url = req.url()?;
    let path = url.path();

    if path == "/" || path.is_empty() {
        return response::root(VERSION);
    }

    if path == "/health" {
        return health(VERSION);
    }

    if path == "/v1/stats" {
        return handle_stats(req, env, ctx).await;
    }

    err(404, "Not found", None)
}

async fn handle_stats(req: Request, env: Env, ctx: worker::Context) -> Result<Response> {
    let url = req.url()?;
    let raw = url
        .query_pairs()
        .find(|(k, _)| k == "username")
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default();

    let user = match Username::new(&raw) {
        Some(u) => u,
        None => return err(400, "Invalid username format", None),
    };

    let cache_key_url = url.to_string();
    let edge_cache = worker::Cache::default();

    // Hit nearest PoP. Bypasses Rust execution & global KV completely on hits.
    if let Ok(Some(cached_resp)) = edge_cache.get(&cache_key_url, false).await {
        worker::console_log!("Edge cache hit for {}", user.as_str());
        return Ok(cached_resp);
    }

    let kv = env.kv("RATE_LIMIT_KV").ok();

    // 2. FAST PATH: Global KV Cache check
    if let Some(ref kv_store) = kv {
        let kv_cache_key = format!("cache:{}", user.as_str());
        if let Ok(Some(cached)) = kv_store.get(&kv_cache_key).text().await {
            if let Ok(stats) = serde_json::from_str::<GitHubStats>(&cached) {
                worker::console_log!("KV cache hit for {}", user.as_str());
                let resp = success(stats.clone(), 10, 60, true)?;

                // Background task: Promote this KV hit to the local Edge Cache
                let cache_url = cache_key_url.clone();
                ctx.wait_until(async move {
                    let cache = worker::Cache::default();
                    if let Ok(cache_resp) = success(stats, 10, 60, true) {
                        let _ = cache.put(cache_url, cache_resp).await;
                    }
                });

                return Ok(resp);
            }
        }
    }

    let token = env
        .secret("GITHUB_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if token.is_empty() {
        return err(500, "Server configuration error", None);
    }

    let client = ClientId::from_req(&req);

    // 3. SLOW PATH: Rate limiting only if we're actually going to hit GitHub
    let (remaining, reset) = if let Some(ref kv_store) = kv {
        let limiter = Limiter::new(kv_store);

        let (global_hit, username_hit) = futures::join!(
            limiter.check_global(&client),
            limiter.check_username_global(&user),
        );

        if global_hit? {
            return err(429, "Too many requests. Slow down.", Some((0, 60)));
        }

        if username_hit? {
            return err(429, "Too many requests for this user. Try again in a minute.", Some((0, 60)));
        }

        let cfg = RateConfig::default();
        match limiter.check_with_info(&client, &user, cfg).await? {
            Some(info) => info,
            None => return err(429, "Rate limit exceeded. Try again in 5 minutes.", Some((0, 300))),
        }
    } else {
        (10u32, 60u64)
    };

    let gh = GitHubClient::new(token);
    let gql = match gh.fetch(&user).await {
        Ok(r) => r,
        Err(e) => {
            worker::console_log!("GitHub error: {:?}", e);
            let msg = e.to_string();
            let code = if msg.contains("rate limit") { 503 } else { 502 };
            let display_msg = if code == 503 {
                "GitHub API rate limit exceeded"
            } else {
                "Failed to fetch data from GitHub"
            };
            return err(code, display_msg, None);
        }
    };

    let gql_user = match gql.data {
        Some(d) => d.user,
        None => return err(404, "User not found", None),
    };

    let contributed: Vec<_> = gql_user
        .contributions_collection
        .commit_contributions_by_repository
        .iter()
        .map(|c| c.repository.clone())
        .collect();

    let (repo_cnt, stars, langs, top) = process_repos(
        &gql_user.repositories.nodes,
        &gql_user.public_repositories.nodes,
        &contributed,
    );

    let stats = build_stats(gql_user, user.as_str(), repo_cnt, stars, langs, top);

    // Create the immediate user response (Instant response over HTTP to the client)
    let resp = success(stats.clone(), remaining, reset, false)?;

    // Background tasks: Store in KV cache and Edge cache without blocking the HTTP response
    let user_str = user.as_str().to_string();
    let cache_url = cache_key_url;
    
    ctx.wait_until(async move {
        // 1. Edge Cache Background Save
        let cache = worker::Cache::default();
        if let Ok(cache_resp) = success(stats.clone(), remaining, reset, false) {
            let _ = cache.put(cache_url, cache_resp).await;
        }

        // 2. Global KV Cache Background Save
        if let Ok(kv_bg) = env.kv("RATE_LIMIT_KV") {
            if let Ok(json) = serde_json::to_string(&stats) {
                let kv_key = format!("cache:{}", user_str);
                if let Ok(builder) = kv_bg.put(&kv_key, &json) {
                    let _ = builder.expiration_ttl(CACHE_TTL).execute().await;
                    worker::console_log!("Cached stats for {}", user_str);
                }
            }
        }
    });

    Ok(resp)
}
