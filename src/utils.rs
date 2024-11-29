use core::fmt;
use serde::{Deserialize, Serialize};

/// This is a generic printable thing. The concrete examples would be:
#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize)]
pub struct PrintableEntity<S>
where
    S: Into<String>,
{
    pub name: S,
    pub description: S,
}

impl<S> fmt::Display for PrintableEntity<S>
where
    S: Into<String> + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.description)
    }
}

pub use PrintableEntity as CommitType;
pub type BundledCommitType<'a> = CommitType<&'a str>;
// pub type UserProvidedType = CommitType<String>;
// pub use PrintableEntity as CommitScope;

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
