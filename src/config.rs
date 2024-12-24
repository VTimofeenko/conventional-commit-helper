use anyhow::Result;
use const_format::formatcp;
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, MAIN_SEPARATOR};

use crate::utils::{UserProvidedCommitScope, UserProvidedCommitType};

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
