use assert_cmd::Command;
use conventional_commit_helper::test_utils::{
    mk_config_full, mk_config_with_scopes_only, mk_config_with_types_only, setup_repo_with_commits,
};
use predicates::prelude::*;
use std::sync::Once;

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

/// Set up a custom repo with a custom config file and check that it's returned
#[test]
fn check_custom_repo_with_config() {
    init_logger();

    // Set up environment
    let dir = assert_fs::TempDir::new().unwrap();
    let _ = setup_repo_with_commits(dir.path(), &["init"]);
    mk_config_with_types_only(dir.path());

    // Setup command
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.arg("type");
    cmd.arg("--debug");
    // Change CWD to the fake repo
    cmd.current_dir(dir.path());

    // Test
    cmd.assert().success();

    cmd.assert().stdout(predicate::str::contains("foo"));
}

/// Set up a custom repo with a custom config file and check that it's returned
#[test]
fn check_custom_repo_with_config_and_scopes() {
    init_logger();

    // Set up environment
    let dir = assert_fs::TempDir::new().unwrap();
    let _ = setup_repo_with_commits(dir.path(), &["init", "foo(qux): quux"]);
    mk_config_with_scopes_only(dir.path());

    // Setup command
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.arg("scope");
    cmd.arg("--debug");
    // Change CWD to the fake repo
    cmd.current_dir(dir.path());

    // Test
    cmd.assert().success();

    // From config
    cmd.assert().stdout(predicate::str::contains("foz"));
    // From history
    cmd.assert().stdout(predicate::str::contains("qux"));
}

/// Sets up a repo with a config and commits and scopes. Checks that everything is as expected
#[test]
fn all_together_now() {
    init_logger();

    // Set up environment
    let dir = assert_fs::TempDir::new().unwrap();
    let _ = setup_repo_with_commits(dir.path(), &["init", "foo(qux): quux"]);
    mk_config_full(dir.path());

    // Test types
    // Setup command
    let mut cmd_types = Command::cargo_bin(BIN_NAME).unwrap();
    cmd_types.arg("type");
    cmd_types.arg("--debug");
    // Change CWD to the fake repo
    cmd_types.current_dir(dir.path());

    // Test
    cmd_types.assert().success();

    // From config
    cmd_types.assert().stdout(predicate::str::contains("foo"));

    // Test scopes
    // Setup command
    let mut cmd_scopes = Command::cargo_bin(BIN_NAME).unwrap();
    cmd_scopes.arg("scope");
    cmd_scopes.arg("--debug");
    // Change CWD to the fake repo
    cmd_scopes.current_dir(dir.path());

    // Test
    cmd_scopes.assert().success();

    // From config
    cmd_scopes.assert().stdout(predicate::str::contains("foz"));
    // From history
    cmd_scopes.assert().stdout(predicate::str::contains("qux"));
}

// Ensure logger is initialized only once for all tests
static INIT: Once = Once::new();

// To be used when neeeded by the tests, otherwise too spammy.
fn init_logger() {
    INIT.call_once(|| {
        env_logger::Builder::new()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true) // Ensures output is test-friendly
            .init();
    });
}
