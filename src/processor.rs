use crate::types::{GqlUser, Repo};
use shared::github::{GitHubLanguage, GitHubStats, MostStarredRepo};
use std::collections::{HashMap, HashSet};

pub fn process_repos(
    private: &[Repo],
    public: &[Repo],
    contributed: &[Repo],
) -> (u32, u32, Vec<(String, u64)>, Option<MostStarredRepo>) {
    let mut seen = HashSet::new();
    let mut lang_bytes: HashMap<String, u64> = HashMap::new();
    let mut total_stars: u32 = 0;
    let mut top: Option<(String, u32, String)> = None;

    for r in private.iter().chain(public.iter()).chain(contributed.iter()) {
        if !seen.insert(r.name.clone()) {
            continue;
        }

        total_stars = total_stars.saturating_add(r.stargazer_count);

        let best = top.as_ref().map(|(_, s, _)| *s).unwrap_or(0);
        if r.stargazer_count > best {
            top = Some((r.name.clone(), r.stargazer_count, r.url.clone()));
        }

        for e in &r.languages.edges {
            *lang_bytes.entry(e.node.name.clone()).or_insert(0) += e.size;
        }
    }

    let cnt = seen.len() as u32;

    let mut langs: Vec<(String, u64)> = lang_bytes.into_iter().collect();
    langs.sort_by(|a, b| b.1.cmp(&a.1));

    let most_starred = top.map(|(n, s, u)| MostStarredRepo {
        name: n,
        stars: s,
        url: u,
    });

    (cnt, total_stars, langs, most_starred)
}

fn calc_pct(bytes: u64, total: u64) -> Option<u32> {
    if total == 0 {
        return None;
    }
    Some(((bytes as f64 / total as f64) * 100.0).round() as u32)
}

pub fn build_stats(
    user: GqlUser,
    username: &str,
    repo_cnt: u32,
    total_stars: u32,
    langs: Vec<(String, u64)>,
    top_repo: Option<MostStarredRepo>,
) -> GitHubStats {
    let total_bytes: u64 = langs.iter().map(|(_, b)| b).sum();

    let languages: Vec<GitHubLanguage> = langs
        .into_iter()
        .filter_map(|(name, bytes)| {
            calc_pct(bytes, total_bytes)
                .filter(|&p| p > 0)
                .map(|pct| GitHubLanguage {
                    name,
                    percentage: pct,
                })
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
