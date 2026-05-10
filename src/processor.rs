use crate::types::{GqlUser, Repo};
use shared::github::{GitHubLanguage, GitHubStats, MostStarredRepo};
use std::collections::{HashMap, HashSet};
use std::cmp::Ordering;

pub fn process_repos<'a>(
    private: &'a [Repo],
    public: &'a [Repo],
    contributed: &'a [Repo],
) -> (u32, u32, Vec<(String, f64)>, Option<MostStarredRepo>) {

    let mut seen = HashSet::new();
    let mut lang_shares: HashMap<&'a str, f64> = HashMap::new();
    let mut total_stars: u32 = 0;
    let mut top: Option<(&'a str, u32, &'a str)> = None;
    let mut repos_with_langs: u32 = 0;

    for r in private.iter().chain(public.iter()).chain(contributed.iter()) {
        if !seen.insert(r.name.as_str()) {
            continue;
        }

        total_stars = total_stars.saturating_add(r.stargazer_count);

        let best = top.as_ref().map(|(_, s, _)| *s).unwrap_or(0);

        if r.stargazer_count > best {
            top = Some((r.name.as_str(), r.stargazer_count, r.url.as_str()));
        }

        let total_repo_bytes: u64 = r.languages.edges.iter().map(|e| e.size).sum();

        if total_repo_bytes == 0 {
            continue;
        }

        repos_with_langs += 1;

        for e in &r.languages.edges {
            let share = e.size as f64 / total_repo_bytes as f64;
            *lang_shares.entry(e.node.name.as_str()).or_insert(0.0) += share;
        }
    }

    let cnt = seen.len() as u32;

    let mut langs: Vec<(String, f64)> = if repos_with_langs > 0 {
        let denom = repos_with_langs as f64;
        lang_shares
            .into_iter()
            .map(|(name, total_share)| (name.to_string(), total_share / denom))
            .collect()
    } else {
        Vec::new()
    };

    langs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

    let most_starred = top.map(|(n, s, u)| MostStarredRepo {
        name: n.to_string(),
        stars: s,
        url: u.to_string(),
    });

    (cnt, total_stars, langs, most_starred)
}

pub fn build_stats(
    user: GqlUser,
    username: &str,
    repo_cnt: u32,
    total_stars: u32,
    langs: Vec<(String, f64)>,
    top_repo: Option<MostStarredRepo>,
) -> GitHubStats {
    let languages: Vec<GitHubLanguage> = langs
        .into_iter()
        .filter_map(|(name, avg_share)| {
            let pct = (avg_share * 100.0).round() as u32;
            if pct > 0 {
                Some(GitHubLanguage { name, percentage: pct })
            } else {
                None
            }
        })
        .collect();

    GitHubStats {
        total_repos: repo_cnt,
        total_contributions: user.contributions_collection
            .contribution_calendar.total_contributions,
        total_stars,
        followers: user.followers.total_count,
        following: user.following.total_count,
        total_commits: user.contributions_collection
            .total_commit_contributions,
        total_prs: user.contributions_collection
            .total_pull_request_contributions,
        total_issues: user.contributions_collection
            .total_issue_contributions,
        account_created_at: user.created_at,
        most_starred_repo: top_repo,
        avatar_url: user.avatar_url,
        display_name: user.name.unwrap_or_else(|| username.to_string()),
        bio: user.bio.unwrap_or_default(),
        languages,
    }
}
