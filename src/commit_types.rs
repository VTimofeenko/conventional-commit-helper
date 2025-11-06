use crate::config::Config;
use crate::utils::PrintableEntity;
use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone, Hash, Ord, PartialOrd)]
pub struct CommitType {
    pub name: String,
    pub description: String,
}

impl PrintableEntity for CommitType {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
}

#[derive(Clone, Debug)]
pub struct CommitTypeRef<'a> {
    pub name: &'a str,
    pub description: &'a str,
}

pub const DEFAULT_COMMIT_TYPES: &[CommitTypeRef] = &[
    CommitTypeRef {
        name: "feat",
        description: "A new feature",
    },
    CommitTypeRef {
        name: "fix",
        description: "A bug fix",
    },
    CommitTypeRef {
        name: "docs",
        description: "Documentation only changes",
    },
    CommitTypeRef {
        name: "style",
        description: "Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc)",
    },
    CommitTypeRef {
        name: "refactor",
        description: "A code change that neither fixes a bug nor adds a feature",
    },
    CommitTypeRef {
        name: "perf",
        description: "A code change that improves performance",
    },
    CommitTypeRef {
        name: "test",
        description: "Adding missing tests or correcting existing tests",
    },
    CommitTypeRef {
        name: "build",
        description: "Changes that affect the build system or external dependencies (example scopes: gulp, broccoli, npm)",
    },
    CommitTypeRef {
        name: "ci",
        description: "Changes to our CI configuration files and scripts (example scopes: Travis, Circle, BrowserStack, SauceLabs)",
    },
    CommitTypeRef {
        name: "chore",
        description: "Other changes that don't modify src or test files",
    },
];

pub fn get_commit_types_from_repo_or_default(config: Option<Config>) -> Result<Vec<CommitType>> {
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

pub fn get_default_commit_types() -> Vec<CommitType> {
    DEFAULT_COMMIT_TYPES
        .iter()
        .map(|c| CommitType {
            name: c.name.to_string(),
            description: c.description.to_string(),
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

        assert_eq!(res.unwrap(), get_default_commit_types())
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
