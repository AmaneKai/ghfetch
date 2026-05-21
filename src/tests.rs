#[cfg(test)]
mod validation_tests {
    use crate::types::Username;

    fn valid(s: &str) -> bool {
        Username::new(s).is_some()
    }

    #[test]
    fn accepts_simple_username() {
        assert!(valid("torvalds"));
    }

    #[test]
    fn accepts_username_with_hyphen() {
        assert!(valid("pring-nt"));
    }

    #[test]
    fn accepts_alphanumeric() {
        assert!(valid("user123"));
    }

    #[test]
    fn rejects_empty() {
        assert!(!valid(""));
    }

    #[test]
    fn rejects_too_long() {
        assert!(!valid(&"a".repeat(40)));
    }

    #[test]
    fn rejects_leading_hyphen() {
        assert!(!valid("-username"));
    }

    #[test]
    fn rejects_trailing_hyphen() {
        assert!(!valid("username-"));
    }

    #[test]
    fn rejects_double_hyphen() {
        assert!(!valid("user--name"));
    }

    #[test]
    fn rejects_special_chars() {
        assert!(!valid("user@name"));
        assert!(!valid("user name"));
        assert!(!valid("user.name"));
        assert!(!valid("<script>"));
    }

    #[test]
    fn normalizes_to_lowercase() {
        let u = Username::new("AmaneKai").unwrap();
        assert_eq!(u.as_str(), "amanekai");
    }

    #[test]
    fn accepts_max_length() {
        assert!(valid(&"a".repeat(39)));
    }

    #[test]
    fn rejects_whitespace_only() {
        assert!(!valid("   "));
    }
}

#[cfg(test)]
mod processor_tests {
    use crate::processor::process_repos;
    use crate::types::{LangConn, LangEdge, LangNode, Owner, Repo};

    fn make_repo(name: &str, stars: u32, owner: &str, langs: Vec<(&str, u64)>) -> Repo {
        Repo {
            name: name.to_string(),
            owner: Owner { login: owner.to_string() },
            stargazer_count: stars,
            url: format!("https://github.com/{owner}/{name}"),
            languages: LangConn {
                edges: langs
                    .into_iter()
                    .map(|(n, s)| LangEdge {
                        size: s,
                        node: LangNode { name: n.to_string() },
                    })
                    .collect(),
            },
        }
    }

    #[test]
    fn counts_unique_repos() {
        let repos = vec![
            make_repo("repo-a", 5, "user", vec![]),
            make_repo("repo-b", 3, "user", vec![]),
        ];
        let (cnt, _, _, _) = process_repos("user", &repos, &[], &[]);
        assert_eq!(cnt, 2);
    }

    #[test]
    fn deduplicates_repos_by_name() {
        let private = vec![make_repo("repo-a", 5, "user", vec![])];
        let public = vec![make_repo("repo-a", 5, "user", vec![])];
        let (cnt, _, _, _) = process_repos("user", &private, &public, &[]);
        assert_eq!(cnt, 1);
    }

    #[test]
    fn sums_stars() {
        let repos = vec![
            make_repo("repo-a", 10, "user", vec![]),
            make_repo("repo-b", 20, "user", vec![]),
        ];
        let (_, stars, _, _) = process_repos("user", &repos, &[], &[]);
        assert_eq!(stars, 30);
    }

    #[test]
    fn finds_most_starred() {
        let repos = vec![
            make_repo("low", 1, "user", vec![]),
            make_repo("high", 99, "user", vec![]),
            make_repo("mid", 50, "user", vec![]),
        ];
        let (_, _, _, top) = process_repos("user", &repos, &[], &[]);
        assert_eq!(top.unwrap().name, "high");
    }

    #[test]
    fn top_repo_must_be_owned() {
        let owned = vec![make_repo("owned", 10, "user", vec![])];
        let contributed = vec![make_repo("other", 100, "someone-else", vec![])];
        let (_, stars, _, top) = process_repos("user", &owned, &[], &contributed);
        
        assert_eq!(stars, 110); // Still counts all stars
        assert_eq!(top.unwrap().name, "owned"); // But top is the owned one
    }

    #[test]
    fn averages_language_shares_across_repos() {
        let repos = vec![
            make_repo("repo-a", 0, "user", vec![("Rust", 1000), ("C", 500)]),
            make_repo("repo-b", 0, "user", vec![("Rust", 500)]),
        ];
        let (_, _, langs, _) = process_repos("user", &repos, &[], &[]);
        let rust = langs.iter().find(|(n, _)| n == "Rust").unwrap();
        let c = langs.iter().find(|(n, _)| n == "C").unwrap();
        assert!((rust.1 - 0.833).abs() < 0.01, "Rust avg share should be ~0.833, got {}", rust.1);
        assert!((c.1 - 0.167).abs() < 0.01, "C avg share should be ~0.167, got {}", c.1);
    }

    #[test]
    fn sorts_languages_by_share_descending() {
        let repos = vec![make_repo(
            "repo",
            0,
            "user",
            vec![("C", 100), ("Rust", 900), ("Python", 500)],
        )];
        let (_, _, langs, _) = process_repos("user", &repos, &[], &[]);
        assert_eq!(langs[0].0, "Rust");
        assert_eq!(langs[1].0, "Python");
        assert_eq!(langs[2].0, "C");
    }

    #[test]
    fn handles_empty_repos() {
        let (cnt, stars, langs, top) = process_repos("user", &[], &[], &[]);
        assert_eq!(cnt, 0);
        assert_eq!(stars, 0);
        assert!(langs.is_empty());
        assert!(top.is_none());
    }

    #[test]
    fn no_overflow_on_large_stars() {
        let repos = vec![
            make_repo("a", u32::MAX, "user", vec![]),
            make_repo("b", u32::MAX, "user", vec![]),
        ];
        let (_, stars, _, _) = process_repos("user", &repos, &[], &[]);
        assert_eq!(stars, u32::MAX);
    }

    #[test]
    fn empty_repos_dont_dilute_language_percentages() {
        let repos = vec![
            make_repo("repo-a", 0, "user", vec![("Rust", 1000)]),
            make_repo("repo-b", 0, "user", vec![]),
        ];
        let (_, _, langs, _) = process_repos("user", &repos, &[], &[]);
        let rust = langs.iter().find(|(n, _)| n == "Rust").unwrap();
        assert!(
            (rust.1 - 1.0).abs() < 0.01,
            "Rust should be 100%, got {}",
            rust.1
        );
    }
}
