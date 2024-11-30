use anyhow::{bail, Result};
use git2::Repository;
use log::debug;
// use std::fs::File;
// use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::utils::{
    try_config_file_in_repo, CommitType, UserProvidedCommitType, DEFAULT_COMMIT_TYPES,
};

fn try_get_commit_types_from_repo_at_path<P>(path: P) -> Result<Option<Vec<UserProvidedCommitType>>>
where
    P: Into<PathBuf> + AsRef<Path> + std::fmt::Debug,
{
    // Try to find repo at location.
    let repo: Repository = match Repository::discover(path) {
        Ok(x) => x,
        Err(err) => match err.code() {
            // No repo -- OK, don't need to search it
            git2::ErrorCode::NotFound => return Ok(None),
            // Return any other error
            _ => bail!(err),
        },
    };

    match try_config_file_in_repo(repo)? {
        Some(config) => {
            debug!("Found config in repo, returning its commit_types");
            Ok(config.commit_types)
        }
        None => {
            debug!("No user-defined commit types found");
            Ok(None)
        }
    }
}

pub fn get_commit_types_from_repo_or_default<P>(path: P) -> Result<Vec<CommitType<String>>>
where
    P: Into<PathBuf> + AsRef<Path> + std::fmt::Debug,
{
    match try_get_commit_types_from_repo_at_path(path)? {
        Some(x) => Ok(x),
        None => Ok(get_default_commit_types()),
    }
}

pub fn get_default_commit_types() -> Vec<CommitType<String>> {
    DEFAULT_COMMIT_TYPES
        .iter()
        .map(|c| {
            // This might be better implemented through From?
            CommitType::<String> {
                name: c.name.to_string(),
                description: c.description.to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::DEFAULT_CONFIG_PATH_IN_REPO;
    use conventional_commit_helper::test_utils::setup_repo_with_commits;
    use indoc::indoc;
    use rstest::{fixture, rstest};
    use std::fs;
    use std::sync::Once;
    use testdir::testdir;

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

    #[fixture]
    fn mk_types() -> String {
        indoc! {r#"
                [types]
                foo = "bar"
                "#}
        .to_string()
    }

    /// Creates a fake config file at target location with commit types
    /// Only produces side-effect of a file.
    fn setup_custom_commit_type_file(tmpdir: &Path) {
        let config_path = tmpdir.join(DEFAULT_CONFIG_PATH_IN_REPO);
        debug!("Setting up config path at {:?}", config_path);
        let _ = fs::create_dir_all(config_path.parent().unwrap());
        fs::write(config_path, mk_types()).unwrap();
    }

    /// Checks that fallback works for various paths
    #[rstest]
    #[case::empty_dir(testdir!())]
    #[case::nonexistent_dir(PathBuf::from("/none"))]
    fn no_repo_no_custom_types(#[case] dir: PathBuf) {
        init_logger();
        let res = try_get_commit_types_from_repo_at_path(dir).unwrap();
        assert!(res.is_none());
    }

    #[rstest]
    fn empty_repo_check_no_custom_types() {
        let dir = testdir!();
        let repo = setup_repo_with_commits(&dir, &["init"]);

        let res = try_get_commit_types_from_repo_at_path(repo.workdir().unwrap());

        assert!(res.unwrap().is_none())
    }

    #[rstest]
    fn empty_repo_check_default_returned() {
        let dir = testdir!();
        let repo = setup_repo_with_commits(&dir, &["init"]);

        let res = get_commit_types_from_repo_or_default(repo.workdir().unwrap());

        assert_eq!(res.unwrap(), DEFAULT_COMMIT_TYPES)
    }

    #[rstest]
    fn empty_repo_with_custom_commit_type() {
        init_logger();
        let dir = testdir!();
        let repo = setup_repo_with_commits(&dir, &["init"]);
        setup_custom_commit_type_file(&dir);

        let res = get_commit_types_from_repo_or_default(repo.workdir().unwrap()).unwrap();

        assert_eq!(res.len(), 1);
        assert_eq!(res.first().unwrap().name, "foo");
    }
}
