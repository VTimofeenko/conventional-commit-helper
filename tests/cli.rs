use assert_cmd::Command;
use conventional_commit_helper::test_utils::{
    mk_config_full, mk_config_with_scopes_only, mk_config_with_types_only, setup_repo_with_commits,
    setup_repo_with_commits_and_files,
};
use predicates::prelude::*;
use std::path::Path;
use std::sync::Once;

use predicate::str::{contains, starts_with};

static BIN_NAME: &str = "conventional-commit-helper"; // Default binary name

/// Ensure that the when run without parameters the program succeeds
#[test]
fn default_run_no_args() {
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.assert().success();

    // The two default types should be present
    for default_type in ["feat", "fix"] {
        cmd.assert().stdout(contains(default_type));
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
    cmd.arg("--debug");
    cmd.arg("type");
    // Change CWD to the fake repo
    cmd.current_dir(dir.path());

    // Test
    cmd.assert().success();

    cmd.assert().stdout(contains("foo"));
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
    cmd.arg("--debug");
    cmd.arg("scope");
    // Change CWD to the fake repo
    cmd.current_dir(dir.path());

    // Test
    cmd.assert().success();

    // From config
    cmd.assert().stdout(contains("foz"));
    // From history
    cmd.assert().stdout(contains("qux"));
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
    cmd_types.arg("--debug");
    cmd_types.arg("type");
    // Change CWD to the fake repo
    cmd_types.current_dir(dir.path());

    // Test
    cmd_types.assert().success();

    // From config
    cmd_types.assert().stdout(contains("foo"));

    // Test scopes
    // Setup command
    let mut cmd_scopes = Command::cargo_bin(BIN_NAME).unwrap();
    cmd_scopes.arg("--debug");
    cmd_scopes.arg("scope");
    // Change CWD to the fake repo
    cmd_scopes.current_dir(dir.path());

    // Test
    cmd_scopes.assert().success();

    // From config
    cmd_scopes.assert().stdout(contains("foz"));
    // From history
    cmd_scopes.assert().stdout(contains("qux"));
}

/// Sets up a repo, creates some fake commits and stages files that overlap with a past scope.
/// Checks that the suggested scope is first in the list despite not being first alphabetically
#[test]
fn valid_scope_is_suggested() {
    init_logger();

    // Set up environment
    let dir = assert_fs::TempDir::new().unwrap();
    let repo = setup_repo_with_commits_and_files(
        dir.path(),
        &["init", "foo(z_bar): quux", "foo(baz): quux"],
        &["init", "one", "two"],
    );
    mk_config_full(dir.path());

    // Test scopes
    // Setup command
    let mut cmd_scopes = Command::cargo_bin(BIN_NAME).unwrap();
    cmd_scopes.arg("--debug");
    cmd_scopes.arg("scope");
    // Change CWD to the fake repo
    cmd_scopes.current_dir(dir.path());

    // Test
    cmd_scopes.assert().success();

    // at first, "baz" is first (alphanum)
    cmd_scopes.assert().stdout(starts_with("baz"));

    // stage files
    let mut index = repo.index().unwrap();
    std::fs::write(dir.join("one"), "test writing").unwrap();
    let _ = index.add_path(Path::new("one")); // File has to be relative to the repo to be committed
    let _ = index.write();

    // now "z_bar" is first as it matches the current files
    cmd_scopes.assert().stdout(starts_with("z_bar"));
}

/// This test validates basic cache manipulations. It does not look into the cache itself.
#[test]
fn cache_ops() {
    init_logger();

    // Set up environment
    let dir = assert_fs::TempDir::new().unwrap();
    let repo_path = dir.path().join("repo");
    let cache_path = dir
        .path()
        .join("conventional-commit-helper/commit_scope_cache.bin");
    let _repo = setup_repo_with_commits_and_files(
        &repo_path,
        &["init", "foo(z_bar): quux", "foo(baz): quux"],
        &["init", "one", "two"],
    );

    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .env("XDG_CACHE_HOME", dir.path())
        .arg("--debug")
        .arg("--repo-path")
        .arg(&repo_path)
        .arg("cache")
        .arg("create")
        .assert()
        .success();

    // Check that cache exists
    assert!(cache_path.exists());

    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .env("XDG_CACHE_HOME", dir.path())
        .arg("--debug")
        .arg("--repo-path")
        .arg(&repo_path)
        .arg("cache")
        .arg("update")
        .assert()
        .success();

    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .env("XDG_CACHE_HOME", dir.path())
        .arg("--debug")
        .arg("--repo-path")
        .arg(&repo_path)
        .arg("cache")
        .arg("drop")
        .assert()
        .success();

    // Check that cache still exists
    assert!(cache_path.exists());
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .env("XDG_CACHE_HOME", dir.path())
        .arg("--debug")
        .arg("--repo-path")
        .arg(repo_path)
        .arg("cache")
        .arg("nuke")
        .assert()
        .success();

    // Check that cache is gone
    assert!(!cache_path.exists());
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
