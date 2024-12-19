use anyhow::Result;
use fancy_regex::Regex;
use git2::{Commit, Repository, Status};
use itertools::any;
use log::debug;
use std::collections::{HashMap, HashSet};

use crate::utils::UserProvidedCommitScope;

/// Things that deal with the repository go here

// As a design decision, I am working with file names and not paths. The key point of this
// structure is to be able to quickly compare two sets of changed files by names. As a first
// approach I will use strings for Levenstein-like distance(?).
//
// If I want to switch over to subpath checking or whatever -- I should probably move this
// structure to hashset of paths.
pub type ChangedFiles = HashSet<String>;

/// Returns the list of changed files
///
/// Using hashset to explicitly denote that there is no order
fn get_changed_files_from_commit(commit: &Commit, repo: &Repository) -> ChangedFiles {
    let mut res = HashSet::new(); // Accumulator object

    let this_commit_tree = match commit.tree() {
        Ok(x) => x,
        Err(e) => {
            debug!("Cannot get the {:?} commit's tree.", commit.id());
            debug!("Error: {:?}", e);
            debug!("Returning no changes");
            return res;
        }
    };

    // no parents <=> initial commit?
    if commit.parent_count() != 0 {
        for parent in commit.parents() {
            let parent_tree = match parent.tree() {
                Ok(t) => t,
                Err(e) => {
                    debug!("Cannot find a tree for the parent {:?}", parent.id());
                    debug!("Error: {:?}", e);
                    debug!("Skipping the parent");
                    continue;
                }
            };
            let diff = match repo.diff_tree_to_tree(
                Some(&parent_tree),
                Some(&this_commit_tree),
                Some(&mut git2::DiffOptions::new()),
            ) {
                Ok(x) => x,
                Err(e) => {
                    debug!(
                        "Cannot find diff from {:?} to {:?}",
                        parent_tree.id(),
                        this_commit_tree.id()
                    );
                    debug!("Error: {:?}", e);
                    debug!("Skipping parent");
                    continue;
                }
            };

            diff.deltas().for_each(|delta| {
                let changed_file = match delta.new_file().path().and_then(|p| p.to_str()) {
                    Some(path) => path.to_string(),
                    None => {
                        debug!("Cannot get the changed file path, probably it's not utf-8");
                        debug!("It will be ignored");
                        return;
                    }
                };
                res.insert(changed_file);
            });
        }
    };
    res
}

/// This function should be called on a repo to get the staged files
///
/// No files staged -- return None
pub fn get_staged_files(repo: &Repository) -> Result<Option<ChangedFiles>> {
    let needed_statuses = [
        Status::INDEX_NEW,                            // new staged files
        Status::INDEX_MODIFIED,                       // files fully staged for commit
        Status::INDEX_DELETED,                        // removed files
        Status::INDEX_RENAMED,                        // renamed files
        Status::INDEX_MODIFIED | Status::WT_MODIFIED, // files partially staged for commit
        Status::INDEX_NEW | Status::WT_MODIFIED,
    ];

    let maybe_paths: HashSet<Option<String>> = repo
        .statuses(None)?
        .iter()
        // Filter only the staged things
        .filter(|x| needed_statuses.contains(&x.status()))
        // .path() may yield None on bad non-utf8 paths
        .map(|x| x.path().map(|p| p.to_string()))
        .collect();

    // If any path is none:
    // 1. Alert user
    // 2. Exclude it from the result
    //
    // Alternative implementation would have used some .map creativity above, but looks like there
    // is no "inspect_none"-like method that would capture all side effect demons in a
    // non-returning bottle.
    if any(&maybe_paths, |opt| opt.is_none()) {
        debug!("Some paths appear to be non-utf8. These are ignored.");
    };

    let paths: ChangedFiles = maybe_paths.into_iter().flatten().collect();

    // debug if no files changed
    if paths.is_empty() {
        debug!("No files staged for commit");
    };
    Ok((!paths.is_empty()).then_some(paths))
}

/// Given a single commit message, tries to find a scope in it
fn get_scope_from_commit_message(message: &str) -> Option<String> {
    // LATER: maybe only show this for very verbose output
    debug!("Checking git commit message {:?}", message);
    // Typically scopes are found in the brackets:
    // refactor(conventional-commit-helper): Change CommitType -> PrintableEntity to make it more generic

    // The regex has:
    //
    // 1. Lookbehind: search for an opening bracket
    // 2. Match any alphanum+space
    // 3. Until a closing bracket is encountered with (optionally) exclamation point (for breaking
    //    changes) and a colon
    //
    // Implementation note:  using fancy regex as it seems to align with my prior knowledge of
    // regexes more and it supports lookarounds
    //
    // Digging the match from a capture group seems excessive
    let regex = Regex::new(r"(?<=\()[\w -]+(?=\)!?:)").unwrap();

    regex
        .find(message)
        .unwrap_or_else(|e| {
            debug!("Error: {:?}", e);
            debug!("Returning None");
            None
        })
        .map(|m| m.as_str().to_string())
}

pub fn get_scopes_x_changes(
    repo: &Repository,
) -> Result<Option<HashMap<UserProvidedCommitScope, ChangedFiles>>> {
    // idea:
    // Have an accumulator
    // Walk through the repo using reflog?
    // For every commit, if there is a scope in the message -- get its diff and append to the
    // accumulator

    let mut revwalk = repo.revwalk()?;
    // Set the walk from the HEAD
    revwalk.push_head()?;

    let res = revwalk.fold(
        // let res = repo.revwalk()?.push_head().iter().fold(
        HashMap::<UserProvidedCommitScope, ChangedFiles>::new(),
        |mut acc, revwalk_entry| {
            match revwalk_entry {
                Ok(oid) => {
                    // Record the scope and the changed files in the accumulator.
                    // If scope does not exist -- insert it
                    // If it exists -- append the changed files to the set

                    // PERF: this looks like a potentially unneeded lookup. If performance starts to suffer --
                    // might be worth refactoring this
                    let commit = repo
                        .find_commit(oid)
                        .expect("This commit really should exist");

                    debug!("Checking commit OID {:?}", commit.id());
                    let scope = get_scope_from_commit_message(
                        commit.summary().expect("Commit should have a message"),
                    );
                    if let Some(extracted_scope) = scope {
                        let scope_obj = UserProvidedCommitScope::new(extracted_scope);
                        let changed_files = get_changed_files_from_commit(&commit, repo);

                        if let Some(existing_changed_files) = acc.get_mut(&scope_obj) {
                            existing_changed_files.extend(changed_files);
                        } else {
                            acc.insert(scope_obj, changed_files);
                        }
                    };
                }
                Err(e) => {
                    debug!("Encountered error {:?}", e);
                    // Short circuit back
                }
            }

            acc
        },
    );

    Ok((!res.is_empty()).then_some(res))
}

#[cfg(test)]
mod tests {
    use super::*;
    use conventional_commit_helper::test_utils::setup_repo_with_commits_and_files;
    use rstest::rstest;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::Path;
    use testdir::testdir;

    #[rstest]
    // Trivial case: nothing staged => nothing returned
    #[case::nothing(None, None, None)]
    // A new file is added and fully staged
    #[case::new_file_stage_full(Some(true), Some("newfile"), Some("newfile"))]
    // A new file is added and partially staged
    #[case::new_file_stage_partial(Some(false), Some("newfile"), Some("newfile"))]
    // An existing file is edited and fully staged
    #[case::edit_file_stage_full(Some(true), Some("existingfile"), Some("existingfile"))]
    // An existing file is edited and partially staged
    #[case::edit_file_stage_partial(Some(false), Some("existingfile"), Some("existingfile"))]
    fn test_staged_files_as_expected(
        #[case] stage_full_file: Option<bool>,
        #[case] filename: Option<&str>,
        #[case] expected: Option<&str>,
    ) {
        // Common setup
        let dir = testdir!();
        let repo = setup_repo_with_commits_and_files(
            &dir,
            &["init", "foo(foz): bar", "foo"], // commit msgs
            &["init", "one", "two"],           // files
        );
        // On None -- just do nothing
        if let Some(edited_file) = filename {
            let edited_file = Path::new(edited_file);

            let mut file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(dir.join(edited_file))
                .unwrap();
            let _ = file.write(b"first line");

            let mut index = repo.index().unwrap();
            assert_eq!(get_staged_files(&repo).unwrap(), None); // Edited but not staged => nothing returned

            // Now stage the file
            let _ = index.add_path(edited_file); // File has to be relative to the repo to be committed
            let _ = index.write();

            if stage_full_file == Some(false) {
                // Write more into the file if stage_full_file is false
                let _ = file.write(b"second line line");
            }
        };

        // Adjust `expected` to match what get_staged_files returns
        let expected = expected.map(|s| HashSet::from([s.to_string()]));

        assert_eq!(get_staged_files(&repo).unwrap(), expected);
    }

    fn mk_set(s: impl IntoIterator<Item = impl AsRef<str>>) -> HashSet<String> {
        HashSet::from_iter(s.into_iter().map(|s| s.as_ref().to_string()))
    }
    #[test]
    fn get_get_changed_files_from_commit() {
        let dir = testdir!();
        let repo = setup_repo_with_commits_and_files(
            &dir,
            &["init", "foo(foz): bar", "foo", "bar"], // commit msgs
            &["init", "one", "two", "two"],           // files
        );

        let reflog = repo.reflog("HEAD").unwrap();

        // Implementation notes:
        //
        // 1. reflog starts with the HEAD and walks backwards
        // 2. The order of changed files should match what's expected.
        let test_res: Vec<HashSet<String>> = reflog
            .iter()
            .map(|x| get_changed_files_from_commit(&repo.find_commit(x.id_new()).unwrap(), &repo))
            .collect();
        let expected: Vec<HashSet<String>> = vec![
            mk_set(["two"]),
            mk_set(["two"]),
            mk_set(["one"]),
            HashSet::new(),
        ];

        assert_eq!(test_res, expected);
    }

    /// Checks extraction of scope from commit message
    #[rstest]
    // Trivial case
    #[case::present("foo(foz): baz", Some("foz"))]
    // Make sure that regex properly captures everything in first brackets it encounters
    #[case::present_multiple_words("foo(foz baz): bar", Some("foz baz"))]
    // Check that only first occurrence is parsed
    #[case::present_multiple_times("foo(bar): baz (foz)", Some("bar"))]
    // Check that random sequence in brackets is not found
    #[case::present_multiple_times("foo baz (foz)", None)]
    // Check that "no scope" is handled correctly
    #[case::absent("foo: baz", None)]
    fn can_extract_scope_from_commit_msg(#[case] msg: &str, #[case] expected: Option<&str>) {
        assert_eq!(
            get_scope_from_commit_message(msg),
            expected.map(String::from)
        )
    }

    /// Naive test. Setup a repo with one change that has a scope and one file.
    #[test]
    fn test_get_scopes_x_files_simple() {
        let dir = testdir!();
        let repo = setup_repo_with_commits_and_files(
            &dir,
            &["init", "foo(foz): bar", "foo"], // commit msgs
            &["init", "one", "two"],           // files
        );

        let res = get_scopes_x_changes(&repo).unwrap();

        let expected: HashMap<UserProvidedCommitScope, ChangedFiles> = HashMap::from([(
            UserProvidedCommitScope::new("foz".to_string()),
            mk_set(["one"]),
        )]);

        assert_eq!(res, Some(expected));
    }

    #[test]
    fn test_get_scopes_x_files_multiple_files_multiple_scopes() {
        let dir = testdir!();
        let repo = setup_repo_with_commits_and_files(
            &dir,
            &[
                "init",
                "foo(foz): bar",
                "foo(foz): bar",
                "foo(baz): bar",
                "foo(baz): bar",
            ], // commit msgs
            &["init", "one", "two", "three", "two"], // files
        );

        let res = get_scopes_x_changes(&repo).unwrap();

        let expected: HashMap<UserProvidedCommitScope, ChangedFiles> = HashMap::from([
            (
                UserProvidedCommitScope::new("foz".to_string()),
                mk_set(["one", "two"]),
            ),
            (
                UserProvidedCommitScope::new("baz".to_string()),
                mk_set(["three", "two"]),
            ),
        ]);

        assert_eq!(res, Some(expected));
    }
}
