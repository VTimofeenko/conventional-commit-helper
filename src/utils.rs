use anyhow::{bail, Context, Result};
use const_format::formatcp;
use core::fmt;
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::MAIN_SEPARATOR;

/// This is a generic printable thing. The concrete examples would be:
#[derive(Debug, Default, Deserialize, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize)]
pub struct PrintableEntity<S>
where
    S: Into<String>,
{
    pub name: S,
    pub description: S,
}

impl<S> PrintableEntity<S>
where
    S: Into<String> + std::default::Default,
{
    pub fn new(name: S) -> Self {
        PrintableEntity {
            name,
            ..Default::default()
        }
    }
}

impl<S> fmt::Display for PrintableEntity<S>
where
    S: Into<String> + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.description)
    }
}

/// Convenience method to turn a key value pair into an instance
impl<S> From<(&String, &String)> for PrintableEntity<S>
where
    S: Into<String> + std::fmt::Display + From<String>,
{
    fn from(tuple: (&String, &String)) -> Self {
        Self {
            name: S::from(tuple.0.to_string()),
            description: S::from(tuple.1.to_string()),
        }
    }
}

// Implement PartialEq for PrintableEntity<String> vs PrintableEntity<&str>
impl PartialEq<PrintableEntity<&str>> for PrintableEntity<String> {
    fn eq(&self, other: &PrintableEntity<&str>) -> bool {
        self.name == other.name && self.description == other.description
    }
}

// Implement PartialEq for PrintableEntity<&str> vs PrintableEntity<String>
impl PartialEq<PrintableEntity<String>> for PrintableEntity<&str> {
    fn eq(&self, other: &PrintableEntity<String>) -> bool {
        self.name == other.name && self.description == other.description
    }
}

pub use PrintableEntity as CommitType;
pub type BundledCommitType<'a> = CommitType<&'a str>;
pub type UserProvidedCommitType = CommitType<String>;

// Maybe could be useful later?
// impl From<BundledCommitType<'_>> for CommitType<String> {
//     fn from(value: BundledCommitType<'_>) -> Self {
//         Self {
//             name: value.name.to_string(),
//             description: value.description.to_string(),
//         }
//     }
// }

pub use PrintableEntity as CommitScope;
pub type UserProvidedCommitScope = CommitScope<String>;

pub const DEFAULT_CONFIG_PATH_IN_REPO: &str =
    formatcp!(".dev{}conventional-commit-helper.toml", MAIN_SEPARATOR);

/// Holds the runtime configuration
#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize)]
pub struct Config {
    // Using "types" to prevent repetition in the config file
    // The code should use commit_types to be less ambiguous about the 'type' word
    #[serde(rename = "types")]
    pub commit_types: Option<Vec<UserProvidedCommitType>>,

    #[serde(rename = "scopes")]
    pub commit_scopes: Option<Vec<UserProvidedCommitScope>>,
}

impl Config {
    /// parse sections as keypairs
    /// i.e.
    /// [types]
    /// foo = bar
    /// should result in Config with one CommitType
    ///
    /// Extracted for easier testing
    fn from_toml(toml_value: HashMap<String, HashMap<String, String>>) -> Result<Self> {
        let commit_types: Option<Vec<UserProvidedCommitType>> = toml_value
            .get("types")
            .map(|x| x.iter().map(UserProvidedCommitType::from).collect());
        let commit_scopes: Option<Vec<UserProvidedCommitType>> = toml_value
            .get("scopes")
            .map(|x| x.iter().map(UserProvidedCommitScope::from).collect());
        Ok(Self {
            commit_types,
            commit_scopes,
        })
    }
    pub fn from_file(path: &Path) -> Result<Option<Self>> {
        match path.exists() {
            true => {
                let content = fs::read_to_string(path)?;

                // This error should be bubbled up
                let raw_config: HashMap<String, HashMap<String, String>> =
                    toml::from_str(&content)?;

                Ok(Some(Self::from_toml(raw_config)?))
            }
            false => Ok(None),
        }
    }

    pub fn try_from_repo(repo: &Repository) -> Result<Option<Self>> {
        Self::from_file(&repo.workdir().unwrap().join(DEFAULT_CONFIG_PATH_IN_REPO))
    }
}

pub const DEFAULT_COMMIT_TYPES: &[BundledCommitType] = &[
    CommitType {
        name: "feat",
        description: "A new feature",
    },
    CommitType {
        name: "fix",
        description: "A bug fix",
    },
    CommitType {
        name: "docs",
        description: "Documentation only changes",
    },
    CommitType {
        name: "chore",
        description: "Other changes that don't modify src or test files",
    },
    CommitType {
        name: "style",
        description: "Changes that do not affect the meaning of the code",
    },
    CommitType {
        name: "refactor",
        description: "A code change that neither fixes a bug nor adds a feature",
    },
    CommitType {
        name: "build",
        description: "Changes that affect the build system or external dependencies",
    },
    CommitType {
        name: "ci",
        description: "Changes to CI configuration files and scripts",
    },
    CommitType {
        name: "perf",
        description: "A code change that improves performance",
    },
    CommitType {
        name: "revert",
        description: "Reverts a previous commit",
    },
    CommitType {
        name: "test",
        description: "Adding missing tests or correcting existing tests",
    },
];

/// Takes a path, returns a repository containing that path.
pub fn repo_from_path(path_in_repo: &Path) -> Result<Repository> {
    let repo = Repository::discover(path_in_repo).context("Failed to discover a repository")?;

    match repo.is_bare() {
        true => bail!("Bare repositories are not supported"),
        false => Ok(repo),
    }
}

pub fn validate_repo(repo: &Repository) -> Result<()> {
    if repo.is_bare() {
        bail!("Bare repositories are not supported");
    };

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;

    /// Make sure that the custom "turn key value" From actually works
    #[test]
    fn test_toml_parsing() {
        let toml_data = indoc! {r#"
                [types]
                foo = "bar"
                [scopes]
                foz = "baz"
                "#};

        let res = Config::from_toml(
            toml::from_str(toml_data).expect("This is a test. Parsing should not explode."),
        );

        let expected = Config {
            commit_types: Some(vec![UserProvidedCommitType {
                name: "foo".to_string(),
                description: "bar".to_string(),
            }]),
            commit_scopes: Some(vec![UserProvidedCommitScope {
                name: "foz".to_string(),
                description: "baz".to_string(),
            }]),
        };

        assert_eq!(res.unwrap(), expected)
    }
}
