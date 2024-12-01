use anyhow::Result;
use log::debug;
use std::path::{Path, PathBuf};

use crate::utils::{Config, UserProvidedCommitScope};

// Plan:
// 1. Allow reading commit scopes from a file
// 2. Get commit scopes from git history
// 3. Implement sorting
// 4. Do the distance thing(?)

/// The main entry point to retrieve commit scopes from a git repository at location
/// This function should not panic.
pub fn try_get_commit_scopes_from_repo_at_path<P>(
    path: P,
) -> Result<Option<Vec<UserProvidedCommitScope>>>
where
    P: Into<PathBuf> + AsRef<Path> + std::fmt::Debug,
{
    match Config::from_repo_at_path(&path)? {
        Some(config) => {
            debug!("Found config in repo, returning its commit_scopes");
            Ok(config.commit_scopes)
        }
        None => {
            debug!("No user-defined commit scopes found");
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use conventional_commit_helper::test_utils::{
        setup_config_file_in_path, setup_repo_with_commits,
    };
    use indoc::indoc;
    use rstest::{fixture, rstest};
    use testdir::testdir;

    #[fixture]
    fn mk_scopes() -> String {
        indoc! {r#"
                [scopes]
                foz = "baz"
                "#}
        .to_string()
    }

    /// Basic test: create a repo + config, check it
    #[rstest]
    fn get_from_repo(mk_scopes: String) {
        let dir = testdir!();
        let _ = setup_repo_with_commits(&dir, &["init"]);
        setup_config_file_in_path(&dir, &mk_scopes);

        let res = try_get_commit_scopes_from_repo_at_path(&dir)
            .unwrap()
            .expect("There should be something returned here");
        assert_eq!(res.len(), 1);
        assert_eq!(res.first().unwrap().name, "foz");
    }
}
