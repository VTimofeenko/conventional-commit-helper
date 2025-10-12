use anyhow::Result;
use log::info;

use crate::config::Config;
use crate::utils::{CommitType, DEFAULT_COMMIT_TYPES};

pub fn get_commit_types_from_repo_or_default(
    config: Option<Config>,
) -> Result<Vec<CommitType<String>>> {
    match config {
        Some(config) => {
            info!("Found config, returning its commit_types");
            Ok(config.commit_types.unwrap_or_else(get_default_commit_types))
        }
        None => {
            info!("No custom commit types found, returning default");
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
    fn empty_repo_check_default_returned() {
        let dir = testdir!();
        let repo = setup_repo_with_commits(&dir, &["init"]);
        let config = Config::load(&repo, None).unwrap();

        let res = get_commit_types_from_repo_or_default(config);

        assert_eq!(res.unwrap(), DEFAULT_COMMIT_TYPES)
    }

    #[rstest]
    fn empty_repo_with_custom_commit_type() {
        init_logger();
        let dir = testdir!();
        let repo = setup_repo_with_commits(&dir, &["init"]);
        // This test should control its own commit types to test
        setup_config_file_in_path(&dir, &mk_types());
        let config = Config::load(&repo, None).unwrap();

        let res = get_commit_types_from_repo_or_default(config).unwrap();

        assert_eq!(res.len(), 1);
        assert_eq!(res.first().unwrap().name, "foo");
    }
}
