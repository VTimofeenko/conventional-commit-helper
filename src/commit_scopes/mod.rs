use anyhow::Result;
use git2::Repository;
use log::debug;
use std::path::{Path, PathBuf};

use crate::utils::{Config, UserProvidedCommitScope};

mod commit;

use commit::get_scope_from_commit_message;

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
    debug!("Looking for scopes in config");
    let scopes_from_config: Option<Vec<UserProvidedCommitScope>> =
        match Config::from_repo_at_path(&path)? {
            Some(config) => {
                debug!("Found config in repo, returning its commit_scopes");
                config.commit_scopes
            }
            None => {
                debug!("No user-defined commit scopes found");
                None
            }
        };

    debug!("Looking for scopes in history");
    let scopes_from_history = get_scopes_from_commit_history(&Repository::discover(path)?)?;

    // This can be written more concisely but I will trade it off for readability
    let res = match (scopes_from_config, scopes_from_history) {
        // Both are none -- return none
        (None, None) => {
            debug!("No scopes found in config or history");
            None
        }
        // One is Some() -- return it
        (Some(x), None) => {
            debug!("Found scopes only in config");
            Some(x)
        }
        (None, Some(y)) => {
            debug!("Found scopes only in history");
            Some(y)
        }
        // Both are Some -- smart merge
        (Some(config_scopes), Some(history_scopes)) => {
            debug!("Found scopes in both history and config");
            debug!("Merging the scopes from git history with the project-specific ones. Project-specific ones win.");
            let known_scope_names: Vec<String> =
                config_scopes.iter().map(|x| x.clone().name).collect();
            let filtered_scopes_from_commit_history = history_scopes
                .iter()
                .filter(|x| !known_scope_names.contains(&x.name))
                .cloned()
                .collect();

            let mut scopes = [config_scopes, filtered_scopes_from_commit_history].concat();
            scopes.sort();

            Some(scopes)
        }
    };

    Ok(res)
}

/// Retrieves matches of scopes from the git history
fn get_scopes_from_commit_history(
    repo: &git2::Repository,
) -> Result<Option<Vec<UserProvidedCommitScope>>> {
    let reflog = repo.reflog("HEAD")?;

    let res: Vec<UserProvidedCommitScope> = reflog
        .iter()
        .filter_map(|entry| -> Option<String> {
            // Just getting an entry.message won't work
            // Messages seem to contain an action and some other stuff
            // which would be pretty hard to parse
            //
            // Alternative approach that is slower(?) but more sound:
            // Get the new OID, locate the commit, find its message.
            let new_id = entry.id_new();
            debug!("Looking for commit {:?}", new_id);
            let target_commit: git2::Commit = repo
                .find_commit(new_id)
                // TODO: this panics
                .expect(
                    "Cannot find commit that should exist. This is probably a bug, see debug log.",
                );

            target_commit
                .message()
                // Leave None in place (so filter_map removes these)
                // If not None -- apply get_scope_from_commit_message
                .map_or_else(|| None, get_scope_from_commit_message)
        })
        // dedup by turning it into a hashset
        .collect::<std::collections::HashSet<String>>()
        .iter()
        // Turn into needed structs
        .map(|x| UserProvidedCommitScope {
            name: x.to_string(),
            description: "".to_string(),
        })
        .collect();

    debug!("Found scopes in commit history: {:?}", res);
    // If result is empty -- None. Some(result) otherwise
    Ok((!res.is_empty()).then_some(res))
}

#[cfg(test)]
mod tests {
    use super::*;
    use conventional_commit_helper::test_utils::{
        mk_config_with_scopes_only, setup_config_file_in_path, setup_repo_with_commits,
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

    /// Setup a repo with commits, check that scopes can be extracted from history
    #[rstest]
    fn get_from_repo_history() {
        let dir = testdir!();
        let repo = setup_repo_with_commits(&dir, &["init", "foo(foz): bar"]);

        let res =
            get_scopes_from_commit_history(&repo).expect("There should be something returned here");

        debug!("{:?}", res);

        assert_eq!(res.clone().unwrap().len(), 1);
        assert_eq!(res.unwrap().first().unwrap().name, "foz");
    }

    /// Ensure that if a scope is present in both history and config -- the one from the config
    /// wins so:
    /// 1. A proper description is displayed
    /// 2. No extra lines are printed
    #[rstest]
    fn check_merge() {
        let dir = testdir!();
        let _ = setup_repo_with_commits(&dir, &["init", "foo(foz): bar"]);
        mk_config_with_scopes_only(&dir);

        let res = try_get_commit_scopes_from_repo_at_path(&dir)
            .unwrap()
            .expect("There should be something returned here");

        debug!("{:?}", res);

        assert_eq!(res.clone().len(), 1);
        assert_eq!(res.clone().first().unwrap().name, "foz");
        assert_eq!(res.first().unwrap().description, "baz");
    }
}
