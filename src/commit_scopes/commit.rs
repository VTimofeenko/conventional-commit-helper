#![allow(dead_code)]
use anyhow::Result;
use git2::{Commit, Repository};
use itertools::any;
use log::debug;
use regex::Regex;
use std::collections::{HashMap, HashSet};

use crate::utils::UserProvidedCommitScope;

/// Things that deal with the repository go here
/// The logic to extract scope x paths will be here. The logic to extract commit messages into
/// scopes should also go here
///
///
/// The plan
///
/// 1. ✓ Learn how to traverse commits back to the beginning
/// 2. ✓ Learn how to get changes for commit
/// 3. ✓ Move "extract scope" logic to this file
/// 4. ✓ Construct scope x file changes mapping
/// 5. ✓ Learn how to get staged files
/// 6. ✓ Devicse some smart distance between staged files and sets of file changes to suggest the
///    best matching scope
/// 7. TODOs
/// 8. No unwraps in non-test code
/// 9. ???
/// 10. PROFIT
///

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

    let tree = commit.tree().unwrap();

    // no parents <=> initial commit?
    if commit.parent_count() != 0 {
        for parent in commit.parents() {
            let parent_tree = parent.tree().unwrap();
            let diff = repo
                .diff_tree_to_tree(
                    Some(&parent_tree),
                    Some(&tree),
                    Some(&mut git2::DiffOptions::new()),
                )
                .unwrap();

            diff.deltas().for_each(|delta| {
                let changed_file = delta
                    .new_file()
                    .path()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
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
    let maybe_paths: HashSet<Option<String>> = repo
        .statuses(None)?
        .iter()
        // Filter only the staged things
        .filter(|x| {
            matches!(
                x.status(),
                git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED
                    | git2::Status::INDEX_DELETED
                    | git2::Status::INDEX_RENAMED
            )
        })
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
pub fn get_scope_from_commit_message(message: &str) -> Option<String> {
    // Typically scopes are found in the brackets:
    // refactor(conventional-commit-helper): Change CommitType -> PrintableEntity to make it more generic
    let re = Regex::new(r"\(([^)]*)\)").unwrap();
    let mat = re.find(message);
    // LATER: maybe only show this for very verbose output
    debug!("Checking git commit message {:?}", message);

    mat.map(|arg0: regex::Match<'_>| {
        // Return the string, except for first and last chars which are brackets
        // This should be faster than capture groups
        // Rust regex does not have look(around|behind)
        let res = regex::Match::as_str(&arg0);
        let result = res[1..res.len() - 1].to_string();

        debug!("Found: {:?}", result);
        result
    })
}

pub fn get_scopes_x_changes(
    repo: &Repository,
) -> Result<Option<HashMap<UserProvidedCommitScope, ChangedFiles>>> {
    // idea:
    // Have an accumulator
    // Walk through the repo using reflog?
    // For every commit, if there is a scope in the message -- get its diff and append to the
    // accumulator
    let mut accumulator = HashMap::<UserProvidedCommitScope, ChangedFiles>::new();

    // TODO: reflog vs rewalk -- latter may expose commits as is without an extra lookup
    repo.reflog("HEAD")?.iter().for_each(|reflog_entry| {
        let commit = repo.find_commit(reflog_entry.id_new()).unwrap();
        let scope =
            get_scope_from_commit_message(commit.message().expect("Commit should have a message"));
        if let Some(extracted_scope) = scope {
            let scope_obj = UserProvidedCommitScope::new(extracted_scope);
            let changed_files = get_changed_files_from_commit(&commit, repo);

            if let Some(existing_changed_files) = accumulator.get_mut(&scope_obj) {
                existing_changed_files.extend(changed_files);
            } else {
                accumulator.insert(scope_obj, changed_files);
            }
        }
    });

    Ok((!accumulator.is_empty()).then_some(accumulator))
}

#[cfg(test)]
mod tests {
    use super::*;
    use conventional_commit_helper::test_utils::setup_repo_with_commits_and_files;
    use rstest::rstest;
    use std::path::Path;
    use testdir::testdir;

    #[test]
    fn test_staged_files_as_expected() {
        let dir = testdir!();
        let repo = setup_repo_with_commits_and_files(
            &dir,
            &["init", "foo(foz): bar", "foo"], // commit msgs
            &["init", "one", "two"],           // files
        );
        let edited_file_name = "somefile";
        let edited_file = Path::new(edited_file_name);

        // Test that none is returned when nothing is edited
        assert_eq!(get_staged_files(&repo).unwrap(), None);

        let mut index = repo.index().unwrap();
        std::fs::write(dir.join(edited_file), "test writing").unwrap();

        // Test that none is returned when nothing is staged
        assert_eq!(get_staged_files(&repo).unwrap(), None);

        let _ = index.add_path(edited_file); // File has to be relative to the repo to be committed
        let _ = index.write();

        debug!("{:?}", get_staged_files(&repo));

        // Check that the edited file is returned
        assert_eq!(
            get_staged_files(&repo).unwrap(),
            Some(HashSet::from([edited_file_name.to_string()]))
        );
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
