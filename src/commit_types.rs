use anyhow::Result;
use git2::Repository;
use log::debug;
// use std::fs::File;
// use std::io::BufReader;

use crate::utils::{CommitType, Config, UserProvidedCommitType, DEFAULT_COMMIT_TYPES};

fn try_get_commit_types_from_repo(
    repo: &Repository,
) -> Result<Option<Vec<UserProvidedCommitType>>> {
    match Config::try_from_repo(repo)? {
        Some(config) => {
            debug!("Found config in repo, returning its commit_types");
            Ok(config.commit_types)
        }
        None => {
            debug!("No user-defined commit types found");
            Ok(None)
        }
    }
}

pub fn get_commit_types_from_repo_or_default(repo: &Repository) -> Result<Vec<CommitType<String>>> {
    match try_get_commit_types_from_repo(repo)? {
        Some(x) => {
            debug!("Found custom commit types, returning them");
            Ok(x)
        }
        None => {
            debug!("No custom commit types found, returning default");
            Ok(get_default_commit_types())
        }
    }
}

pub fn get_default_commit_types() -> Vec<CommitType<String>> {
    DEFAULT_COMMIT_TYPES
        .iter()
        .map(|c| {
            // This might be better implemented through From?
            CommitType::<String> {
                name: c.name.to_string(),
                description: c.description.to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use conventional_commit_helper::test_utils::{
        setup_config_file_in_path, setup_repo_with_commits,
    };
    use indoc::indoc;
    use rstest::{fixture, rstest};
    use std::sync::Once;
    use testdir::testdir;

    // Ensure logger is initialized only once for all tests
    static INIT: Once = Once::new();

    // To be used when neeeded by the tests, otherwise too spammy.
    fn init_logger() {
        INIT.call_once(|| {
            env_logger::Builder::new()
                .filter_level(log::LevelFilter::Debug)
                .is_test(true) // Ensures output is test-friendly
                .init();
        });
    }

    #[fixture]
    fn mk_types() -> String {
        indoc! {r#"
                [types]
                foo = "bar"
                "#}
        .to_string()
    }

    #[rstest]
    fn empty_repo_check_no_custom_types() {
        let dir = testdir!();
        let repo = setup_repo_with_commits(&dir, &["init"]);

        let res = try_get_commit_types_from_repo(&repo);

        assert!(res.unwrap().is_none())
    }

    #[rstest]
    fn empty_repo_check_default_returned() {
        let dir = testdir!();
        let repo = setup_repo_with_commits(&dir, &["init"]);

        let res = get_commit_types_from_repo_or_default(&repo);

        assert_eq!(res.unwrap(), DEFAULT_COMMIT_TYPES)
    }

    #[rstest]
    fn empty_repo_with_custom_commit_type() {
        init_logger();
        let dir = testdir!();
        let repo = setup_repo_with_commits(&dir, &["init"]);
        // This test should control its own commit types to test
        setup_config_file_in_path(&dir, &mk_types());

        let res = get_commit_types_from_repo_or_default(&repo).unwrap();

        assert_eq!(res.len(), 1);
        assert_eq!(res.first().unwrap().name, "foo");
    }
}
