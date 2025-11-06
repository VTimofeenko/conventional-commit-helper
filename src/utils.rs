use anyhow::{bail, Context, Result};
use git2::Repository;
use std::path::Path;

pub trait PrintableEntity {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}

/// Takes a path, returns a repository containing that path.
pub fn repo_from_path(path_in_repo: &Path) -> Result<Repository> {
    let repo = Repository::discover(path_in_repo).context("Failed to discover a repository")?;

    match repo.is_bare() {
        true => bail!("Bare repositories are not supported"),
        false => Ok(repo),
    }
}

pub fn validate_repo(repo: &Repository) -> Result<()> {
    if repo.is_bare() {
        bail!("Bare repositories are not supported");
    };

    Ok(())
}
