use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct LanguageResult {
    pub name: String,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct RepoInfo {
    pub name: String,
    pub stars: u32,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct DashboardData {
    pub name: String,
    pub avatar_url: String,
    pub followers: u32,
    pub following: u32,
    pub total_contributions: u32,
    pub total_commits: u32,
    pub repo_count: u32,
    pub languages: Vec<LanguageResult>,
    pub most_starred_repo: Option<RepoInfo>,
}

#[derive(Deserialize)]
pub struct GqlResp {
    pub data: Option<GqlData>,
}

#[derive(Deserialize)]
pub struct GqlData {
    pub user: GqlUser,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GqlUser {
    pub avatar_url: String,
    pub name: Option<String>,
    pub followers: CountConn,
    pub following: CountConn,
    pub contributions_collection: ContribColl,
    pub repositories: RepoConn,
}

#[derive(Deserialize)]
pub struct CountConn {
    #[serde(rename = "totalCount")]
    pub total_count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContribColl {
    pub contribution_calendar: Calendar,
    pub total_commit_contributions: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub total_contributions: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoConn {
    pub total_count: u32,
    pub nodes: Vec<Repo>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Repo {
    pub name: String,
    pub stargazerCount: u32, // Serde will map this to stargazerCount in JSON
    pub url: String,
    pub languages: LangConn,
}

#[derive(Deserialize, Clone)]
pub struct LangConn {
    pub edges: Vec<LangEdge>,
}

#[derive(Deserialize, Clone)]
pub struct LangEdge {
    pub size: u64,
    pub node: LangNode,
}

#[derive(Deserialize, Clone)]
pub struct LangNode {
    pub name: String,
}

#[derive(Debug)]
pub struct Username(String);

impl Username {
    pub fn new(s: &str) -> Option<Self> {
        if s.is_empty() { return None; }
        Some(Self(s.to_string()))
    }
    pub fn as_str(&self) -> &str { &self.0 }
}
