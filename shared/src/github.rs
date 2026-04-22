use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitHubLanguage {
    pub name: String,
    pub percentage: u32,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitHubStats {
    pub total_repos: u32,
    pub total_contributions: u32,
    pub languages: Vec<GitHubLanguage>,
    pub total_stars: u32,
    pub followers: u32,
    pub following: u32,
    pub total_commits: u32,
    pub total_prs: u32,
    pub total_issues: u32,
    pub account_created_at: String,
    pub most_starred_repo: Option<MostStarredRepo>,
    pub avatar_url: String,
    pub display_name: String,
    pub bio: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct MostStarredRepo {
    pub name: String,
    pub stars: u32,
    pub url: String,
}
