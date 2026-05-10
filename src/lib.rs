use worker::event;

mod types;
mod rate_limit;
mod github;
mod processor;
mod response;
mod tests;

use types::{ClientId, RateConfig};
use rate_limit::Limiter;
use github::GitHubClient;
use processor::{build_stats, process_repos};
use response::{cors_preflight, err, health, success};
use shared::github::GitHubStats;

use crate::processor::StatsInput;

const CACHE_TTL: u64 = 300; // 5 minutes
const VERSION: &str = "1.0.0";

#[event(fetch)]
pub async fn main(
    req: worker::Request,
    env: worker::Env,
    ctx: worker::Context,
) -> Result<worker::Response, worker::Error> {
    if req.method() == worker::Method::Options {
        return cors_preflight();
}

    if req.method() != worker::Method::Get {
        return err(405, "Method not allowed", None);
    }

    let url = req.url()?;
    let path = url.path();

    // Root - API info
    if path == "/" || path.is_empty() {
        return response::root(VERSION);
    }

    // Health check
    if path == "/health" {
        return health(VERSION);
    }

    // Only accept /v1/stats
    if path != "/v1/stats" {
        return err(404, "Not found", None);
    }

    let raw = url
        .query_pairs()
        .find(|(k, _)| k == "username")
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default();


    let user = match types::Username::new(&raw) {
        Some(u) => u,
        None => return err(400, "Invalid username format", None),
    };

    let kv = env.kv("RATE_LIMIT_KV").ok();
    let token = env
        .secret("GITHUB_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if token.is_empty() {
        return err(500, "Server configuration error", None);
    }

    let client = ClientId::from_req(&req);

    // Rate limiting
    let (remaining, reset) = if let Some(ref kv_store) = kv {
        let limiter = Limiter::new(kv_store);
        let cache_key = format!("cache:{}", user.as_str());

        let (global_hit, username_hit, cached) = futures::join!(
            limiter.check_global(&client),
            limiter.check_username_global(&user),
            kv_store.get(&cache_key).text(),
        );

        if global_hit? {
            return err(429, "Too many requests. Slow down.", Some((0, 60)));
        }

        if username_hit? {
            return err(429, "Too many requests for this user. Try again in a minute.", Some((0, 60)));
        }

        if let Ok(Some(cached_str)) = cached {
            if let Ok(stats) = serde_json::from_str::<GitHubStats>(&cached_str) {
                worker::console_log!("Cache hit for {}", user.as_str());
                return success(stats, 10, 60, true);
            }
        }

        let cfg = RateConfig::default();
        match limiter.check_with_info(&client, &user, cfg).await? {
            Some(info) => info,
            None => return err(429, "Rate limit exceeded. Try again in 5 minutes.", Some((0, 300))),
        }
    } else {
        (10u32, 60u64)
    };

    // Cache check
    if let Some(ref kv_store) = kv {
        let cache_key = format!("cache:{}", user.as_str());
        if let Ok(Some(cached)) = kv_store.get(&cache_key).text().await {
            if let Ok(stats) = serde_json::from_str::<GitHubStats>(&cached) {
                worker::console_log!("Cache hit for {}", user.as_str());
                return success(stats, remaining, reset, true);
            }
        }
    }

    let gh = GitHubClient::new(token);
    let gql = match gh.fetch(&user).await {
        Ok(r) => r,
        Err(e) => {
            worker::console_log!("GitHub error: {:?}", e);
            let msg = e.to_string();
            let code = if msg.contains("rate limit") { 503 } else { 502 };
            return err(code, &msg, None);
        }
    };

    let gql_user = match gql.data {
        Some(d) => d.user,
        None => return err(404, "User not found", None),
    };

    let crate::types::GqlUser {
        avatar_url,
        name,
        bio,
        created_at,
        followers,
        following,
        mut contributions_collection,
        repositories,
        public_repositories,
    } = gql_user;

    let contributed: Vec<_> = contributions_collection
        .commit_contributions_by_repository
        .drain(..)
        .map(|c| c.repository)
        .collect();

    let all_repos = repositories.nodes.into_iter()
        .chain(public_repositories.nodes)
        .chain(contributed);

    let (repo_cnt, stars, langs, top) = process_repos(all_repos);

    let stats = build_stats(StatsInput {
            username: user.as_str().to_string(), 
            avatar_url, 
            name, 
            bio, 
            created_at, 
            followers, 
            following, 
            contributions: contributions_collection, 
            repo_cnt, 
            total_stars: stars, 
            langs, 
            top_repo: top, 
        }
    );

    // Store in cache
    if let Some(ref kv_store) = kv {
        if let Ok(json) = serde_json::to_string(&stats) {
            let cache_key = format!("cache:{}", user.as_str());
            let put_future = kv_store
                .put(&cache_key, &json)?
                .expiration_ttl(CACHE_TTL)
                .execute();

            ctx.wait_until(async move {
                let _ = put_future.await;
                worker::console_log!("Cached stats for {}", cache_key);
            });
        }
    }

    success(stats, remaining, reset, false)
}
