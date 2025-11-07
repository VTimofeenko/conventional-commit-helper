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

pub mod time {

    #[cfg(not(test))]
    use chrono::{DateTime, Utc};

    #[cfg(not(test))]

    pub fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[cfg(test)]
    pub use mock_time::now;

    #[cfg(test)]
    pub mod mock_time {
        use chrono::{DateTime, Utc};
        use std::cell::RefCell;

        thread_local! {
            static MOCK_TIME: RefCell<Option<DateTime<Utc>>> = RefCell::new(None);
        }

        pub fn now() -> DateTime<Utc> {
            MOCK_TIME.with(|time| {
                if let Some(mock) = *time.borrow() {
                    mock
                } else {
                    Utc::now()
                }
            })
        }

        pub fn set(time: DateTime<Utc>) {
            MOCK_TIME.with(|mock_time| {
                *mock_time.borrow_mut() = Some(time);
            });
        }

        #[allow(dead_code)]
        pub fn clear() {
            MOCK_TIME.with(|mock_time| {
                *mock_time.borrow_mut() = None;
            });
        }
    }
}
