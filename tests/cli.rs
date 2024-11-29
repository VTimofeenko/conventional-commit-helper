use assert_cmd::Command;
use predicates::prelude::*;

static BIN_NAME: &str = "conventional-commit-helper"; // Default binary name

/// Ensure that the when run without parameters the program succeeds
#[test]
fn default_run_no_args() {
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.assert().success();

    // The two default types should be present
    for default_type in ["feat", "fix"] {
        cmd.assert().stdout(predicate::str::contains(default_type));
    }
}
