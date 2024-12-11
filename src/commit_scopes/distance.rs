use itertools::sorted;
use log::debug;

use crate::utils::UserProvidedCommitScope;

use super::commit::ChangedFiles;
use std::cmp::Ordering::{Equal, Greater, Less};
use std::collections::{HashMap, HashSet};

/// This file contains the logic to help with calculating the most appropriate scope
///
/// The idea put broadly is:
/// Given a set of files staged for commit("set_1") and a set of pairs {(scope: { files previously
/// changed as part of that scope }) ... }("set_2"), which of the "scope" entries' files set is
/// closest to the set of currently staged files set_1?
///
/// The plan is:
/// 1. ✓ Write closest match search naive implementation
/// 2. ✓ Use code from here to actually guess the scope
/// 3. Consider making the searc path-aware
/// 4. Maybe generalize the algorithm to turn into a metric (see below)
///
/// Some thoughts on the implementation:
///
/// S = "scope", F = "set of files matching the scope":
/// set_2 := { (S_1, F_1), ... }
///
/// Trivial cases:
///
/// (math symbols used the way author's grug brain remembers/understands them)
///
/// 1. Complete match:
///
///     ∃F_i ∈ set_2 | set_1 = F_i => S_i
///
/// 2. Staged files are a subset of one of the scopes' sets:
///
///     ∃F_i ∈ set_2 | set_1 ⊆ F_i
///       & (∀F_j F_j != F_i | |set_1 - F_i| < |set_1 - F_j| )
///     => S_i
///
/// 3. If staged files are a superset of one of the scopes' sets, then find the set F which has the
///    biggest overlap with F without considering the partial order(is that the right term?) of the
///    F sets.
///
///     ∃F_i ∈ set_2 | set_1 ⊃ F_i
///       & (∀F_j F_j != F_i | |set_1 - F_i| < |set_1 - F_j| )
///     => S_i
///
/// 4. If set_1 does not overlap with any F_i, then find the biggest overlap
///
///     ∃F_i ∈ set_2 | set_1 ⊅ F_i & set_1 ⊄ F_i
///       & (∀F_j F_j != F_i | |set_1 - F_i| < |set_1 - F_j| )
///     => S_i
///
/// This all rolls up to a simple statement "find me all sets F from set_2 that have the largest
/// overlap with set_1".
///
/// Naive tie breaker: if multiple scopes are found -- alphanumeric sort
///
/// Path awareness logic:
///
/// Since sets of paths in the form of { "foo", "bar/baz", "quot/qux/qoz", ... }, the _path_
/// similarity could be considered for selecting the closest neighbor. I.e. if I changed something
/// in "bar/baz" and I have a scope that has changed something in "bar/foz" -- they might match.
///
/// How about this:
///
/// 1. Construct a prefix tree from the all F sets in set_2. The nodes will be annotated with the scopes
/// 2. Construct a prefix tree from set_1
/// 3. Find scopes that annotating nodes from the overlap
///
/// ?
///
/// Degenerate case would need to be handled -- there could be two or more scopes with exactly one
/// overlap...
///
/// On the technical level this could be implemented by prefix trees. Note to self -- don't just
/// split the string on "/", poor unfortunate souls on Windows won't be happy.
///
/// Stretch goals:
///
/// Turn "closest" neighbor into more of a metric so that the list of scopes can be sorted in terms
/// of how close of a match it is.
///
/// Naive approach is to recursively sort the set_2 by comparing set_1 with set_2 / {(S_i, F_i)}
/// where S_i was chosen on previous step. This is probably horrible performance-wise.
///

fn find_by_overlap(
    staged_files: ChangedFiles,
    scope_set: HashMap<UserProvidedCommitScope, ChangedFiles>,
) -> HashSet<UserProvidedCommitScope> {
    scope_set
        .iter()
        // Go through the set, constructing pairs (scope, count_of_overlapping_items)
        .map(|(scope, set)| {
            let overlap = staged_files.intersection(set).count();
            (scope, overlap)
        })
        .fold(
            // Iterate over the constructed pairs, keeping only pairs with the largest overlap
            // Don't keep the pairs with an overlap = 0 since they don't have any intersection
            (0, HashSet::new()), // Seed argument
            |(max_overlap, mut result), (scope, overlap)| match overlap.cmp(&0) {
                Less => unreachable!(), // Cannot be. Overlap is always >= 0
                Equal => (max_overlap, result),
                Greater => match overlap.cmp(&max_overlap) {
                    Less => (max_overlap, result),
                    Equal => {
                        result.insert(scope.clone());
                        (max_overlap, result)
                    }
                    Greater => (overlap, HashSet::from([scope.clone()])),
                },
            },
        )
        .1 // return only the aggregated hashset
}

pub fn find_closest_neighbor(
    staged_files: ChangedFiles,
    scope_set: HashMap<UserProvidedCommitScope, ChangedFiles>,
) -> Option<UserProvidedCommitScope> {
    debug!("Staged files: {:?}", staged_files);
    let res = find_by_overlap(staged_files, scope_set);

    sorted(res)
        .collect::<Vec<UserProvidedCommitScope>>()
        .first()
        .cloned()
}

#[cfg(test)]
mod test {
    use rstest::{fixture, rstest};

    use super::*;
    use std::collections::{HashMap, HashSet};

    #[fixture]
    fn needle() -> UserProvidedCommitScope {
        UserProvidedCommitScope::new("needle".to_string())
    }

    #[fixture]
    fn cruft() -> UserProvidedCommitScope {
        UserProvidedCommitScope::new("cruft".to_string())
    }

    /// Test exactly equal search result is found in a trivial case
    #[fixture]
    fn staged_files() -> ChangedFiles {
        HashSet::from(["foo".to_string(), "bar".to_string()])
    }

    #[rstest]
    fn test_exact_match_trivial_one_result(
        needle: UserProvidedCommitScope,
        cruft: UserProvidedCommitScope,
        staged_files: ChangedFiles,
    ) {
        let haystack = HashMap::from([
            // This should be found
            (needle.clone(), staged_files.clone()),
            // This is a distraction
            (cruft.clone(), HashSet::from(["baz".to_string()])),
            // This is a distraction
            (cruft, HashSet::from(["baz".to_string()])),
        ]);

        assert_eq!(
            find_closest_neighbor(staged_files, haystack).unwrap(),
            needle
        )
    }

    /// Test exactly equal search result + tie breaker
    #[rstest]
    fn test_exact_match_multiple_results(
        needle: UserProvidedCommitScope,
        cruft: UserProvidedCommitScope,
        staged_files: ChangedFiles,
    ) {
        let mut other_needle = needle.clone();
        // Exposes implementation a bit by using alphanum lower name
        other_needle.name = "z_needle".to_string();

        let haystack = HashMap::from([
            // This should be found
            (needle.clone(), staged_files.clone()),
            // This is a distraction, should not be found
            (other_needle, staged_files.clone()),
            // This is a distraction
            (cruft.clone(), HashSet::from(["baz".to_string()])),
        ]);

        assert_eq!(
            find_closest_neighbor(staged_files, haystack).unwrap(),
            needle
        )
    }

    #[rstest]
    fn test_staged_is_subset_match_one_result(
        needle: UserProvidedCommitScope,
        cruft: UserProvidedCommitScope,
        staged_files: ChangedFiles,
    ) {
        let mut old_changed_files = staged_files.clone();
        old_changed_files.extend(vec!["qux".to_string()]);
        let haystack = HashMap::from([
            // This should be found
            (needle.clone(), old_changed_files),
            // This is a distraction
            (cruft.clone(), HashSet::from(["baz".to_string()])),
        ]);

        assert_eq!(find_closest_neighbor(staged_files, haystack), Some(needle))
    }

    #[rstest]
    fn test_staged_is_superset_match_one_result(
        needle: UserProvidedCommitScope,
        cruft: UserProvidedCommitScope,
        staged_files: ChangedFiles,
    ) {
        let mut old_changed_files = staged_files.clone();
        let _ = old_changed_files.remove("bar");
        let haystack = HashMap::from([
            // This should be found
            (needle.clone(), old_changed_files),
            // This is a distraction
            (cruft.clone(), HashSet::from(["baz".to_string()])),
        ]);

        assert_eq!(find_closest_neighbor(staged_files, haystack), Some(needle))
    }

    #[rstest]
    fn test_staged_partial_overlap_match_one_result(
        needle: UserProvidedCommitScope,
        cruft: UserProvidedCommitScope,
        staged_files: ChangedFiles,
    ) {
        let mut old_changed_files = staged_files.clone();
        // Remove one element, add a random one not in the original staged_files
        let _ = old_changed_files.remove("bar");
        old_changed_files.extend(["baz".to_string()]);

        // Test the test
        assert!(old_changed_files.intersection(&staged_files).count() == 1);
        assert!(
            old_changed_files
                .symmetric_difference(&staged_files)
                .count()
                == 2
        ); // 1 from old, 1 from new

        let haystack = HashMap::from([
            // This should be found
            (needle.clone(), old_changed_files),
            // This is a distraction
            (cruft.clone(), HashSet::from(["baz".to_string()])),
        ]);

        assert_eq!(find_closest_neighbor(staged_files, haystack), Some(needle));
    }

    #[rstest]
    fn test_staged_no_overlap_no_result(
        needle: UserProvidedCommitScope,
        cruft: UserProvidedCommitScope,
        staged_files: ChangedFiles,
    ) {
        let cruft_files = HashSet::from(["qux".to_string()]);

        // Test the test
        assert!(cruft_files.intersection(&staged_files).count() == 0);

        let haystack = HashMap::from([
            (needle.clone(), cruft_files.clone()),
            (cruft.clone(), cruft_files),
        ]);

        assert_eq!(find_closest_neighbor(staged_files, haystack), None);
    }
}
