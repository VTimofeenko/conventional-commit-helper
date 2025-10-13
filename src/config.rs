use anyhow::{Ok, Result};
use const_format::formatcp;
use directories::ProjectDirs;
use git2::Repository;
use itertools::Itertools;
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

use crate::utils::{UserProvidedCommitScope, UserProvidedCommitType};

pub const DEFAULT_CONFIG_PATH_IN_REPO: &str =
    formatcp!(".dev{}conventional-commit-helper.toml", MAIN_SEPARATOR);
const CONFIG_FILE_NAME: &str = "conventional-commit-helper.toml";

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Serialize, Default)]
pub struct GeneralConfig {
    pub scopes: Option<GeneralScopeConfig>,
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Serialize, Default)]
pub struct GeneralScopeConfig {
    pub ignored: Option<Vec<String>>,
}

/// Holds the runtime configuration
#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Serialize, Default)]
pub struct Config {
    // Using "types" to prevent repetition in the config file
    // The code should use commit_types to be less ambiguous about the 'type' word
    #[serde(rename = "types")]
    pub commit_types: Option<Vec<UserProvidedCommitType>>,

    #[serde(rename = "scopes")]
    pub commit_scopes: Option<Vec<UserProvidedCommitScope>>,

    pub general: Option<GeneralConfig>,
}

/// Used internally to parse the file
#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Serialize)]
struct ReadConfig {
    #[serde(rename = "types")]
    commit_types: Option<HashMap<String, String>>,

    #[serde(rename = "scopes")]
    commit_scopes: Option<HashMap<String, String>>,

    general: Option<GeneralConfig>,
}

impl Config {
    /// parse sections as keypairs
    /// i.e.
    /// [types]
    /// foo = bar
    /// should result in Config with one CommitType
    ///
    /// Extracted for easier testing
    fn from_str(toml_str: &str) -> Result<Self> {
        let initial_result: ReadConfig = toml::from_str(toml_str)?;
        let commit_types: Option<Vec<UserProvidedCommitType>> = initial_result
            .commit_types
            .map(|x| x.iter().map(UserProvidedCommitType::from).collect());
        let commit_scopes: Option<Vec<UserProvidedCommitType>> = initial_result
            .commit_scopes
            .map(|x| x.iter().map(UserProvidedCommitScope::from).collect());

        Ok(Self {
            commit_scopes,
            commit_types,
            general: initial_result.general,
        })
    }

    pub fn from_file(path: &Path) -> Result<Option<Self>> {
        match path.exists() {
            true => {
                let content = fs::read_to_string(path)?;

                Ok(Some(Self::from_str(&content)?))
            }
            false => Ok(None),
        }
    }

    fn get_global_config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "vtimofeenko", "conventional-commit-helper")
            .map(|proj_dirs| proj_dirs.config_dir().join(CONFIG_FILE_NAME))
    }

    fn merge(self, other: Self) -> Self {
        let commit_types = self
            .commit_types
            .into_iter()
            .flatten()
            .chain(other.commit_types.into_iter().flatten())
            .unique()
            .collect();

        let commit_scopes = self
            .commit_scopes
            .into_iter()
            .flatten()
            .chain(other.commit_scopes.into_iter().flatten())
            .unique()
            .collect();

        let general = self.general.or(other.general);

        Self {
            commit_types: Some(commit_types),
            commit_scopes: Some(commit_scopes),
            general,
        }
    }

    pub fn load(repo: &Repository, from_path: Option<PathBuf>) -> Result<Option<Self>> {
        if let Some(path) = from_path {
            debug!("Loading config from path: {:?}", path);
            return Self::from_file(&path);
        }

        let repo_config =
            Self::from_file(&repo.workdir().unwrap().join(DEFAULT_CONFIG_PATH_IN_REPO))?;

        let global_config_path = Self::get_global_config_path();
        let global_config = if let Some(path) = global_config_path {
            Self::from_file(&path)?
        } else {
            None
        };

        match (repo_config, global_config) {
            (Some(repo), Some(global)) => Ok(Some(repo.merge(global))),
            (Some(repo), None) => Ok(Some(repo)),
            (None, Some(global)) => Ok(Some(global)),
            (None, None) => Ok(None),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;

    /// Make sure that the custom "turn key value" From actually works
    #[test]
    fn test_toml_parsing() {
        let toml_str = indoc! {r#"
                [types]
                foo = "bar"
                [scopes]
                foz = "baz"
                "#};

        let res = Config::from_str(toml_str);

        let expected = Config {
            commit_types: Some(vec![UserProvidedCommitType {
                name: "foo".to_string(),
                description: "bar".to_string(),
            }]),
            commit_scopes: Some(vec![UserProvidedCommitScope {
                name: "foz".to_string(),
                description: "baz".to_string(),
            }]),
            general: None,
        };

        assert_eq!(res.unwrap(), expected)
    }

    #[test]
    fn test_general_settings() {
        let toml_str = indoc! {r#"
            [general]
            scopes.ignored = ["foo", "bar"]
                "#};
        let config: Config = Config::from_str(toml_str).unwrap();

        assert_eq!(
            config.general.unwrap().scopes.unwrap().ignored.unwrap(),
            vec!["foo", "bar"]
        )
    }

    #[test]
    fn test_config_merge() {
        let repo_config = Config {
            commit_types: Some(vec![UserProvidedCommitType {
                name: "foo".to_string(),
                description: "bar".to_string(),
            }]),
            commit_scopes: Some(vec![UserProvidedCommitScope {
                name: "foz".to_string(),
                description: "baz".to_string(),
            }]),
            general: None,
        };

        let global_config = Config {
            commit_types: Some(vec![UserProvidedCommitType {
                name: "foo".to_string(),
                description: "bar".to_string(),
            }]),
            commit_scopes: Some(vec![UserProvidedCommitScope {
                name: "global".to_string(),
                description: "global".to_string(),
            }]),
            general: None,
        };

        let merged = repo_config.merge(global_config);

        let expected = Config {
            commit_types: Some(vec![UserProvidedCommitType {
                name: "foo".to_string(),
                description: "bar".to_string(),
            }]),
            commit_scopes: Some(vec![
                UserProvidedCommitScope {
                    name: "foz".to_string(),
                    description: "baz".to_string(),
                },
                UserProvidedCommitScope {
                    name: "global".to_string(),
                    description: "global".to_string(),
                },
            ]),
            general: None,
        };

        assert_eq!(merged, expected);
    }
}
