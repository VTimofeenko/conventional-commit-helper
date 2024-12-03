use log::debug;
use regex::Regex;
/// Given a single commit message, tries to find a scope in it
pub fn get_scope_from_commit_message(message: &str) -> Option<String> {
    // Typically scopes are found in the brackets:
    // refactor(conventional-commit-helper): Change CommitType -> PrintableEntity to make it more generic
    let re = Regex::new(r"\(([^)]*)\)").unwrap();
    let mat = re.find(message);
    // TODO: maybe only show this for very verbose output
    debug!("Checking git commit message {:?}", message);

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
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
}
