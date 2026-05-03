use crate::types::*;
use worker::{Headers, Method, Request, RequestInit};
use std::collections::HashMap;

const API_URL: &str = "https://api.github.com/graphql";
const QUERY: &str = "
query($u: String!) {
  user(login: $u) {
    avatarUrl name
    followers { totalCount }
    following { totalCount }
    contributionsCollection {
      contributionCalendar { totalContributions }
      totalCommitContributions
    }
    repositories(
      first: 100, 
      ownerAffiliations: [OWNER, COLLABORATOR], 
      orderBy: {field: STARGAZERS, direction: DESC}
    ) {
      totalCount
      nodes {
        name stargazerCount url
        languages(first: 10, orderBy: {field: SIZE, direction: DESC}) {
          edges { size node { name } }
        }
      }
    }
  }
}";

pub struct GitHubClient {
    token: String,
}

impl GitHubClient {
    pub fn new(token: String) -> Self {
        Self { token }
    }
    
    pub async fn fetch(&self, user: &Username) -> Result<DashboardData, worker::Error> {
        let body = serde_json::json!({
            "query": QUERY,
            "variables": { "u": user.as_str() }
        });
        
        let mut init = RequestInit::new();
        init.with_method(Method::Post).with_body(Some(body.to_string().into()));
        
        let h = Headers::new();
        h.set("Authorization", &format!("Bearer {}", self.token))?;
        h.set("Content-Type", "application/json")?;
        h.set("User-Agent", "portfolio-worker/1.0")?;
        init.with_headers(h);
        
        let req = Request::new_with_init(API_URL, &init)?;
        let mut resp = worker::Fetch::Request(req).send().await?;
        
        let raw: GqlResp = resp.json().await?;
        let user_data = raw.data.ok_or_else(|| worker::Error::RustError("No data".into()))?.user;
        let repos = user_data.repositories.nodes;

        let languages = self.calculate_smart_languages(&repos);

        let most_starred_repo = repos.first().map(|r| RepoInfo {
            name: r.name.clone(),
            stars: r.stargazer_count, // Using snake_case here
            url: r.url.clone(),
        });

        Ok(DashboardData {
            name: user_data.name.unwrap_or_else(|| user.as_str().to_string()),
            avatar_url: user_data.avatar_url,
            followers: user_data.followers.total_count,
            following: user_data.following.total_count,
            total_contributions: user_data.contributions_collection.contribution_calendar.total_contributions,
            total_commits: user_data.contributions_collection.total_commit_contributions,
            repo_count: user_data.repositories.total_count,
            languages,
            most_starred_repo,
        })
    }

    fn calculate_smart_languages(&self, repos: &[Repo]) -> Vec<LanguageResult> {
        let mut lang_shares: HashMap<String, f64> = HashMap::new();
        let mut valid_repo_count = 0;

        for repo in repos {
            let total_size: u64 = repo.languages.edges.iter().map(|e| e.size).sum();
            if total_size == 0 { continue; }
            
            valid_repo_count += 1;

            for edge in &repo.languages.edges {
                let share = edge.size as f64 / total_size as f64;
                *lang_shares.entry(edge.node.name.clone()).or_insert(0.0) += share;
            }
        }

        if valid_repo_count == 0 { return Vec::new(); }

        let mut stats: Vec<LanguageResult> = lang_shares
            .into_iter()
            .map(|(name, total_share)| LanguageResult {
                name,
                percentage: (total_share / valid_repo_count as f64) * 100.0,
            })
            .collect();

        stats.sort_by(|a, b| b.percentage.partial_cmp(&a.percentage).unwrap());
        stats
    }
}
