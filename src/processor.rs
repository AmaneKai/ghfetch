use crate::types::{ContribColl, CountConn, Repo};
use shared::github::{GitHubLanguage, GitHubStats, MostStarredRepo};
use std::collections::{HashMap, HashSet};
use std::cmp::Ordering;

pub fn process_repos(
    repos: impl Iterator<Item = Repo>
) -> (u32, u32, Vec<(String, f64)>, Option<MostStarredRepo>) {
    let mut seen = HashSet::new();
    let mut lang_shares: HashMap<String, f64> = HashMap::new();
    let mut total_stars: u32 = 0;
    let mut top: Option<(String, u32, String)> = None;
    let mut repos_with_langs: u32 = 0;

    for r in repos {
        if !seen.insert(r.name.clone()) {
            continue;
        }

        total_stars = total_stars.saturating_add(r.stargazer_count);

        let best = top.as_ref().map(|(_, s, _)| *s).unwrap_or(0);
        if r.stargazer_count > best {
            top = Some((r.name.clone(), r.stargazer_count, r.url.clone()));
        }

        let total_repo_bytes: u64 = r.languages.edges.iter().map(|e| e.size).sum();
        if total_repo_bytes == 0 {
            continue;
        }

        repos_with_langs += 1;

        for e in &r.languages.edges {
            let share = e.size as f64 / total_repo_bytes as f64;
            *lang_shares.entry(e.node.name.clone()).or_insert(0.0) += share;
        }
    }

    let cnt = seen.len() as u32;

    let mut langs: Vec<(String, f64)> = if repos_with_langs > 0 {
        let denom = repos_with_langs as f64;

        lang_shares
            .into_iter()
            .map(|(name, total_share)| (name, total_share / denom))
            .collect()
    } else {
        Vec::new()
    };

    langs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

    let most_starred = top.map(|(n, s, u)| MostStarredRepo {
        name: n,
        stars: s,
        url: u,
    });

    (cnt, total_stars, langs, most_starred)
}

pub struct StatsInput {
    pub username: String,
    pub avatar_url: String,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub created_at: String,
    pub followers: CountConn,
    pub following: CountConn,
    pub contributions: ContribColl,
    pub repo_cnt: u32,
    pub total_stars: u32,
    pub langs: Vec<(String, f64)>,
    pub top_repo: Option<MostStarredRepo>,
}

pub fn build_stats(input: StatsInput) -> GitHubStats {
    let languages: Vec<GitHubLanguage> = input.langs
        .into_iter()
        .filter_map(|(name, avg_share)| {
            let pct = (avg_share * 100.0).round() as u32;
            if pct > 0 { Some(GitHubLanguage { name, percentage: pct }) } else { None }
        })
        .collect();

     GitHubStats {
        total_repos: input.repo_cnt,
        total_contributions: input.contributions.contribution_calendar.total_contributions,
        total_stars: input.total_stars,
        followers: input.followers.total_count,
        following: input.following.total_count,
        total_commits: input.contributions.total_commit_contributions,
        total_prs: input.contributions.total_pull_request_contributions,
        total_issues: input.contributions.total_issue_contributions,
        account_created_at: input.created_at,
        most_starred_repo: input.top_repo,
        avatar_url: input.avatar_url,
        display_name: input.name.unwrap_or_else(|| input.username.clone()),
        bio: input.bio.unwrap_or_default(),
        languages,
    }
}
