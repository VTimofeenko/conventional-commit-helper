use anyhow::Result;
use git2::Repository;
use itertools::sorted;
use log::debug;

use crate::utils::{Config, UserProvidedCommitScope};

mod commit;

use self::commit::{get_scopes_x_changes, get_staged_files};
use self::distance::find_closest_neighbor;

mod distance;

/// The main entry point to retrieve commit scopes from a git repository at location
/// This function should not panic.
pub fn try_get_commit_scopes_from_repo(
    repo: &Repository,
) -> Result<Option<Vec<UserProvidedCommitScope>>> {
    debug!("Looking for scopes in config");
    let scopes_from_config: Option<Vec<UserProvidedCommitScope>> =
        match Config::try_from_repo(&repo)? {
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
    // This needs to return pairs (scope, { changed_files })
    let scopes_from_history = get_scopes_x_changes(&repo)?;

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
            // There's no need to sort this, no scopes_from_history found
            Some(x)
        }
        (None, Some(history_scopes)) => {
            debug!("Found scopes only in history");

            let mut scopes =
                sorted(history_scopes.keys().cloned()).collect::<Vec<UserProvidedCommitScope>>();

            // check the current staged changes, push closest match to the front
            if let Some(staged_files) = get_staged_files(&repo)? {
                let matched_scope = find_closest_neighbor(staged_files, history_scopes);

                match matched_scope {
                    Some(matched_scope) => {
                        debug!("Found a scope matching '{:?}'", matched_scope);
                        scopes = push_to_first(scopes, matched_scope);
                    }
                    None => {
                        debug!("No scope matches currently staged files");
                    }
                };
            }

            Some(scopes)
        }
        // Both are Some -- smart merge
        (Some(config_scopes), Some(history_scopes)) => {
            debug!("Found scopes in both history and config");
            debug!("Merging the scopes from git history with the project-specific ones. Project-specific ones win.");
            let known_scope_names: Vec<String> =
                config_scopes.iter().map(|x| x.clone().name).collect();
            let filtered_scopes_from_commit_history = history_scopes
                .keys()
                .filter(|x| !known_scope_names.contains(&x.name))
                .cloned()
                .collect();

            let mut scopes = [config_scopes, filtered_scopes_from_commit_history].concat();
            scopes.sort();

            // Now, I can check the currently staged files and push the needed scope to the front.
            if let Some(staged_files) = get_staged_files(&repo)? {
                let matched_scope = find_closest_neighbor(staged_files, history_scopes);

                match matched_scope {
                    Some(matched_scope) => {
                        debug!("Found a scope matching '{:?}'", matched_scope);
                        scopes = push_to_first(scopes, matched_scope);
                    }
                    None => {
                        debug!("No scope matches currently staged files");
                    }
                };
            }

            // check the current staged changes, push closest neighbor to the front
            Some(scopes)
        }
    };

    Ok(res)
}

fn push_to_first<T: Ord>(mut v: Vec<T>, first: T) -> Vec<T> {
    if let Some(index) = v.iter().position(|s| s == &first) {
        v.remove(index);
        v.insert(0, first);
    }

    v
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
        let repo = setup_repo_with_commits(&dir, &["init"]);
        setup_config_file_in_path(&dir, &mk_scopes);

        let res = try_get_commit_scopes_from_repo(&repo)
            .unwrap()
            .expect("There should be something returned here");
        assert_eq!(res.len(), 1);
        assert_eq!(res.first().unwrap().name, "foz");
    }

    /// Ensure that if a scope is present in both history and config -- the one from the config
    /// wins so:
    /// 1. A proper description is displayed
    /// 2. No extra lines are printed
    #[rstest]
    fn check_merge() {
        let dir = testdir!();
        let repo = setup_repo_with_commits(&dir, &["init", "foo(foz): bar"]);
        mk_config_with_scopes_only(&dir);

        let res = try_get_commit_scopes_from_repo(&repo)
            .unwrap()
            .expect("There should be something returned here");

        debug!("{:?}", res);

        assert_eq!(res.clone().len(), 1);
        assert_eq!(res.clone().first().unwrap().name, "foz");
        assert_eq!(res.first().unwrap().description, "baz");
    }
}
