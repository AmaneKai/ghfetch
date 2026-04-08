use worker::event;

mod types;
mod validation;
mod rate_limit;
mod github;
mod processor;
mod response;

use types::{ClientId, RateConfig};
use rate_limit::Limiter;
use github::GitHubClient;
use processor::{build_stats, process_repos};
use response::{cors_preflight, err, success};

#[event(fetch)]
pub async fn main(
    req: worker::Request,
    env: worker::Env,
    _ctx: worker::Context,
) -> Result<worker::Response, worker::Error> {
    if req.method() == worker::Method::Options {
        return cors_preflight();
    }

    if req.method() != worker::Method::Get {
        return err(405, "Method not allowed");
    }

    let url = req.url()?;
    let raw = url
        .query_pairs()
        .find(|(k, _)| k == "username")
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default();

    let user = match validation::valid_username(&raw) {
        Some(u) => u,
        None => return err(400, "Invalid username format"),
    };

    let kv = env.kv("RATE_LIMIT_KV").ok();
    let token = env
        .secret("GITHUB_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if token.is_empty() {
        return err(500, "Server configuration error");
    }

    let client = ClientId::from_req(&req);

    if let Some(ref kv_store) = kv {
        let limiter = Limiter::new(kv_store);

        if limiter.check_global(&client).await? {
            return err(429, "Too many requests. Slow down.");
        }

        let cfg = RateConfig::default();
        if !limiter.check(&client, &user, cfg).await? {
            return err(429, "Rate limit exceeded. Try again in 5 minutes.");
        }
    }

    let gh = GitHubClient::new(token);
    let gql = match gh.fetch(&user).await {
        Ok(r) => r,
        Err(e) => {
            worker::console_log!("GitHub error: {:?}", e);
            let msg = e.to_string();
            let code = if msg.contains("rate limit") { 503 } else { 502 };
            return err(code, &msg);
        }
    };

    let gql_user = match gql.data {
        Some(d) => d.user,
        None => return err(404, "User not found"),
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

    success(stats)
}
