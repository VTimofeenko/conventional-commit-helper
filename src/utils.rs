use anyhow::{bail, Context, Result};
use core::fmt;
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::path::Path;

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
