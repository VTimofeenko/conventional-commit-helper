// I want to speed up the lookups of scopes x files when suggesting a commit.
//
// This is a first take on this feature with following non-functional characteristics:
//
// 1. Cache lifecycle:
//     - Creation is explicit by using a dedicated subcommand ($bin cache create)
//     - Updates are emergent (recreate the whole darn thing)
//     - Destruction is explicit ($bin cache drop $repo_path)
//     - Destruction should have a mode to nuke the whole cache ($bin cache nuke)
//
// 2. Cache usage:
//     - before trying to scan through commit history, the commit_scopes logic will look for cache
//     - if cache exists -- it will be assumed to be usable as is and not updated
//
//     => cache is read-only from non-`cache` subcommand
//
// 3. Cache content:
//     - One cache for all repositories
//     - Data model:
//
//         <path to repo> OtM <scopes> OtM <changed files>
//
//         repos don't have any relationship to each other, so this is basically a forest of
//         isolated trees. Repo will be identified by the path.
//
//         Repo path: identification of the repository by path is not ideal and may break when
//         dealing with symlinks or what have you
//
//         In the initial approach, cache is dropped for the whole repo, so I don't need to track
//         individual commits. Later I might change this.
//
//         This data model maps well to what logic in `commit.rs` does currently and should be
//         easier to implement.
//
//         For future I might also consider moving to <path to repo> OtM <changed files> OtM <scopes>.
//
//
// 4. Misc:
//     - Cache is to be stored centrally in $XDG_CACHE_HOME
//         Potential alternative: store it in `.git/` dir
//
//         Pros:
//             - Self-contained with the repo
//             - Does not rely on env variable
//         Cons:
//             - I am not sure how "stable" implanting the cache into .git would be in the sense
//               of "how do I prevent collisions in future"
//
// First approach will use `serde`+`bincode` to store cache on disk. I have used serde before,
// should be easier to get started

use anyhow::{bail, Context, Result};
use directories::ProjectDirs;
use git2::Repository;
use log::{debug, trace};
use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::commit_scopes::commit::{get_scopes_x_changes, ChangedFiles};
use crate::utils::UserProvidedCommitScope;

// Data Structures for the Cache
#[derive(Serialize, Deserialize, Debug)]
pub struct CacheEntry {
    pub scopes: HashMap<UserProvidedCommitScope, ChangedFiles>,
}

/// Repo identifier in the cache.
///
/// Path to the repository seems like a good first approach.
type RepoID = PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Cache {
    // Mapping of <repo path> OtM <cache entry>
    pub entries: HashMap<RepoID, CacheEntry>,
}

impl Cache {
    /// Returns None if cache does not exist
    pub fn load() -> Result<Self> {
        let cache_path = get_cache_path()?;
        if cache_path.exists() {
            let data = std::fs::read(cache_path)?;
            let cache: Cache = bincode::deserialize(&data)?;
            Ok(cache)
        } else {
            bail!("Cache does not exist")
        }
    }

    pub fn lock() -> Result<()> {
        trace!("Acquiring lock on the cache");
        let cache_path = get_cache_path()?;
        let options = file_lock::FileOptions::new().write(true).create(true);
        let _ = file_lock::FileLock::lock(&cache_path, false, options)
            .context("Failed to acquire cache file lock")?;

        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let cache_path = get_cache_path()?;
        let data = bincode::serialize(self)?;
        std::fs::write(cache_path, data)?;
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get_scopes_for_repo(
        &self,
        repo: &Repository,
    ) -> Option<HashMap<UserProvidedCommitScope, ChangedFiles>> {
        self.entries
            .get(&get_repo_id(repo))
            .map(|x| x.scopes.clone())
    }
}

const CACHE_FILE: &str = "commit_scope_cache.bin";

/// Retrieve the cache path.
/// Should be in XDG_CACHE_HOME.
fn get_cache_path() -> Result<PathBuf> {
    debug!("Looking for the cache");
    if let Some(proj_dirs) = ProjectDirs::from("com", "vtimofeenko", "conventional-commit-helper") {
        let cache_dir = proj_dirs.cache_dir();
        let res = cache_dir.join(CACHE_FILE);
        trace!("Cache path: '{:?}'", res);
        Ok(res)
    } else {
        bail!("Unable to get cache directory from XDG")
    }
}

fn get_repo_id(repo: &Repository) -> RepoID {
    repo.path().parent().expect("Parent of repo's path should always exist unless the repo is bare. This might be a bug").to_path_buf()
}

/// Create the cache. It makes very little sense to create just an empty cache, so takes a repo.
pub fn create_cache() -> Result<()> {
    // Error hear means "cannot determine cache location" => error out, don't do anything.
    println!("Creating the cache");
    let cache_path = get_cache_path()?;

    // Create parent directory if it does not exist
    if let Some(parent) = cache_path.parent() {
        if !parent.exists() {
            println!("Creating the parent dir to contain the cache");
            std::fs::create_dir_all(parent)?;
        }
    }

    // Create an empty cache
    if !cache_path.exists() {
        println!("Creating empty cache");
        let cache = Cache::new();
        cache.save()?;
    }

    println!("Cache created at {}", cache_path.to_string_lossy());
    Ok(())
}

/// Update the cache for specific repo
pub fn update_cache_for_repo(repo: &Repository) -> Result<()> {
    let repo_id = get_repo_id(repo);
    println!("Updating the scope cache for repo '{:?}'", repo_id);

    Cache::lock()?;

    // Load the cache
    let mut cache = Cache::load()?;

    debug!("Getting scopes x changes from the repo");
    let scopes_changes = get_scopes_x_changes(repo)?;

    match scopes_changes {
        Some(scopes_changes) => {
            println!("Writing scopes x changes into the cache");
            cache.entries.insert(
                repo_id,
                CacheEntry {
                    scopes: scopes_changes,
                },
            );
        }
        None => {
            bail!("No scopes detected in the repo")
        }
    };

    cache.save()?;
    println!("Cache saved");
    Ok(())
}

/// Drop cache for individual repo
pub fn drop_cache_for_repo(repo: &Repository) -> Result<()> {
    let repo_id = get_repo_id(repo);
    println!("Dropping the scope cache for repo '{:?}'", repo_id);

    Cache::lock()?;

    // Load the cache
    let mut cache = Cache::load()?;

    // If the entry exists, drop it from cache.
    match cache.entries.remove(&repo_id) {
        Some(_) => println!("Dropped the cache for repo at '{:?}'", repo.path()),
        None => println!(
            "Cache for repo at '{:?}' does not exist, not doing a thing",
            repo.path()
        ),
    };

    cache.save()?;

    Ok(())
}

pub fn nuke_cache() -> Result<()> {
    println!("Destroying the whole cache");
    let cache_path = get_cache_path()?;
    match cache_path.exists() {
        true => {
            std::fs::remove_file(cache_path)?;
            println!("Cache is no more. It ceased to be.");
        }
        false => println!("Cache does not exist"),
    }
    Ok(())
}
