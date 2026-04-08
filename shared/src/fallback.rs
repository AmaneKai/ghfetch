use crate::github::{GitHubLanguage, GitHubStats, MostStarredRepo};

pub fn fallback_stats() -> GitHubStats {
    GitHubStats {
        total_repos: 42,
        total_contributions: 1842,
        total_stars: 128,
        followers: 24,
        following: 12,
        total_commits: 1200,
        total_prs: 38,
        total_issues: 15,
        account_created_at: "2021-01-01T00:00:00Z".into(),
        avatar_url: "https://avatars.githubusercontent.com/u/9919?v=4".into(),
        display_name: "Carlos Ranara".into(),
        bio: "CS student obsessed with Rust.".into(),
        most_starred_repo: Some(MostStarredRepo {
            name: "carlosranara.com".into(),
            stars: 42,
            url: "https://github.com/Floranaras/carlosranara.com".into(),
        }),
        languages: vec![
            GitHubLanguage {
                name: "Rust".into(),
                percentage: 62,
                color: "rose-pine-foam".into(),
            },
            GitHubLanguage {
                name: "TypeScript".into(),
                percentage: 21,
                color: "rose-pine-gold".into(),
            },
            GitHubLanguage {
                name: "C".into(),
                percentage: 17,
                color: "rose-pine-iris".into(),
            },
        ],
    }
}
