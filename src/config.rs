use anyhow::{Ok, Result};
use const_format::formatcp;
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, MAIN_SEPARATOR};

use crate::utils::{UserProvidedCommitScope, UserProvidedCommitType};

pub const DEFAULT_CONFIG_PATH_IN_REPO: &str =
    formatcp!(".dev{}conventional-commit-helper.toml", MAIN_SEPARATOR);

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Serialize)]
pub struct GeneralConfig {
    scopes: Option<GeneralScopeConfig>,
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Serialize)]
struct GeneralScopeConfig {
    ignored: Option<Vec<String>>,
}

/// Holds the runtime configuration
#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Serialize)]
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

    pub fn try_from_repo(repo: &Repository) -> Result<Option<Self>> {
        Self::from_file(&repo.workdir().unwrap().join(DEFAULT_CONFIG_PATH_IN_REPO))
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
}
