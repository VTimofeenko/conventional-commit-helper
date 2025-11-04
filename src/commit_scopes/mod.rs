use dialoguer::Confirm;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hash;

use anyhow::Result;
use git2::Repository;
use itertools::sorted;
use log::{debug, info, warn};

use crate::cache::{update_cache_for_repo, Cache};
use crate::config::{Config, RegenerateOnStale};
use crate::utils::UserProvidedCommitScope;

pub mod commit;

use self::commit::{get_scopes_x_changes, get_staged_files};
use self::distance::find_closest_neighbor;

mod distance;

const TTL: u64 = 86400; // 24 hours

/// The main entry point to retrieve commit scopes from a git repository at location
/// This function should not panic.
pub fn try_get_commit_scopes_from_repo(
    repo: &Repository,
    config: Option<Config>,
) -> Result<Option<Vec<UserProvidedCommitScope>>> {
    debug!("Looking for scopes in config");
    let mut hasher = DefaultHasher::new();
    config.hash(&mut hasher);

    let ignored_scopes = config
        .as_ref()
        .and_then(|c| c.general.as_ref())
        .and_then(|g| g.scopes.as_ref())
        .and_then(|s| s.ignored.clone());

    let disable_history_search = config
        .as_ref()
        .and_then(|c| c.general.as_ref())
        .and_then(|g| g.scopes.as_ref())
        .and_then(|s| s.disable_history_search)
        .unwrap_or(false);

    let scopes_from_config = config.as_ref().and_then(|c| c.commit_scopes.clone());

    let scopes_from_config = scopes_from_config.map(|scopes| {
        scopes
            .into_iter()
            .filter(|scope| {
                ignored_scopes
                    .as_ref()
                    .map_or(true, |ignored| !ignored.contains(&scope.name))
            })
            .collect()
    });

    // Look up scopes for the repo in the cache
    // Possible options:
    // 1. Cache failed to load/does not exist -- log error and fall back to history
    // 2. Cache loaded OK but does not have entry for current repo -- log and fall back
    // 3. Cache loaded OK and has entry for current repo -- use that entry
    let scopes_from_cache = match Cache::load() {
        Ok(cache) => {
            info!("Loading scopes from cache");
            if let Some(entry) = cache.get_scopes_for_repo(repo) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();
                let head_commit_hash = repo
                    .head()?
                    .target()
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "HEAD reference has no target. Are there commits in this repository?"
                        )
                    })?
                    .to_string();

                if now - entry.timestamp < TTL || entry.head_commit_hash == head_commit_hash {
                    debug!("Cache is valid");
                    return Ok(Some(entry.scopes.keys().cloned().collect::<Vec<_>>()));
                } else {
                    info!("Cache is stale");

                    let regenerate_on_stale = config
                        .as_ref()
                        .map(|c| c.cache.regenerate_on_stale.clone())
                        .unwrap_or_default();

                    match regenerate_on_stale {
                        RegenerateOnStale::Always => {
                            info!("Regenerating cache");
                            update_cache_for_repo(repo)?;
                            let scopes = get_scopes_x_changes(repo)?;
                            Some(scopes.unwrap_or_default())
                        }
                        RegenerateOnStale::Prompt => {
                            if Confirm::new()
                                .with_prompt("Cache is stale. Regenerate?")
                                .interact()?
                            {
                                info!("Regenerating cache");
                                update_cache_for_repo(repo)?;
                                let scopes = get_scopes_x_changes(repo)?;
                                Some(scopes.unwrap_or_default())
                            } else {
                                None
                            }
                        }
                        RegenerateOnStale::Never => {
                            info!("Not regenerating cache");
                            None
                        }
                    }
                }
            } else {
                None
            }
        }
        Err(e) => {
            warn!("Cache could not be loaded because of {:?}", e);
            None
        }
    };

    let other_scopes = if disable_history_search {
        debug!("Config setting disabled search in history");
        None
    } else {
        scopes_from_cache.or_else(|| {
            warn!(
                "Git history scope lookups are a bit slow. Consider using the cache (see --help)"
            );
            info!("Falling back to searching scopes in history");
            get_scopes_x_changes(repo).unwrap_or(None)
        })
    };

    let other_scopes = other_scopes.map(|scopes| {
        scopes
            .into_iter()
            .filter(|(scope, _)| {
                ignored_scopes
                    .as_ref()
                    .map_or(true, |ignored| !ignored.contains(&scope.name))
            })
            .collect::<HashMap<_, _>>()
    });

    // This can be written more concisely but I will trade it off for readability
    let res = match (scopes_from_config, other_scopes) {
        // Both are none -- return none
        (None, None) => {
            info!("No scopes found in config or history");
            None
        }
        // One is Some() -- return it
        (Some(x), None) => {
            info!("Found scopes only in config");
            // There's no need to sort this, no scopes_from_history found
            Some(x)
        }
        (None, Some(history_scopes)) => {
            debug!("Found scopes only in history or cache");

            let mut scopes =
                sorted(history_scopes.keys().cloned()).collect::<Vec<UserProvidedCommitScope>>();

            // check the current staged changes, push closest match to the front
            if let Some(staged_files) = get_staged_files(repo)? {
                let matched_scope = find_closest_neighbor(staged_files, history_scopes);

                match matched_scope {
                    Some(matched_scope) => {
                        info!("Found a scope matching '{:?}'", matched_scope);
                        scopes = push_to_first(scopes, matched_scope);
                    }
                    None => {
                        info!("No scope matches currently staged files");
                    }
                };
            }

            Some(scopes)
        }
        // Both are Some -- smart merge
        (Some(config_scopes), Some(history_scopes)) => {
            info!("Found scopes in both history and config");
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
            if let Some(staged_files) = get_staged_files(repo)? {
                let matched_scope = find_closest_neighbor(staged_files, history_scopes);

                match matched_scope {
                    Some(matched_scope) => {
                        info!("Found a scope matching '{:?}'", matched_scope);
                        scopes = push_to_first(scopes, matched_scope);
                    }
                    None => {
                        info!("No scope matches currently staged files");
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
        let config = Config::load(&repo, None).unwrap();

        let res = try_get_commit_scopes_from_repo(&repo, config)
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
        let config = Config::load(&repo, None).unwrap();

        let res = try_get_commit_scopes_from_repo(&repo, config)
            .unwrap()
            .expect("There should be something returned here");

        debug!("{:?}", res);

        assert_eq!(res.clone().len(), 1);
        assert_eq!(res.clone().first().unwrap().name, "foz");
        assert_eq!(res.first().unwrap().description, "baz");
    }
}
