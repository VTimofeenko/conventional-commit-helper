use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use log::{debug, info};
use std::path::PathBuf;

use self::commit_types::get_default_commit_types;
use self::config::Config;
use self::utils::{repo_from_path, validate_repo, PrintableEntity};

mod cache;
mod commit_scopes;
mod commit_types;
mod config;
mod utils;

#[derive(Subcommand, Debug)]
enum CacheCommand {
    /// Creates the cache for a repo
    Create,
    /// Updates the cache for a repo
    Update,
    /// Drops cache for a repo
    Drop,
    /// Deletes the whole cache
    Nuke,
    /// Shows the content of the cache
    Show,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Cache operations
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
    /// Show commit types
    Type {
        /// Print output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Show commit scopes
    Scope {
        /// Print output in JSON format
        #[arg(long)]
        json: bool,
    },
}

/// Tiny helper for conventional commits (https://www.conventionalcommits.org).
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the non-bare git repository.
    #[arg(long, default_value = ".")]
    repo_path: PathBuf,

    /// Path to a custom config file
    #[arg(long)]
    config: Option<PathBuf>,

    #[command(flatten)]
    verbose: Verbosity,

    /// Command to execute
    #[command(subcommand)]
    command: Option<Command>,
}

fn default_print<S>(output: Vec<PrintableEntity<S>>)
where
    S: std::fmt::Display,
    std::string::String: std::convert::From<S>,
{
    output.iter().for_each(|x| println!("{}", x))
}

fn json_print<S>(output: Vec<PrintableEntity<S>>) -> anyhow::Result<()>
where
    S: serde::Serialize,
    std::string::String: std::convert::From<S>,
{
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();

    debug!("Launched with args: {:?}", args);

    // Handle no given command. This should be done first so nothing is really validated.
    let Some(command) = args.command else {
        info!("Running in default mode, just printing the types");
        default_print(get_default_commit_types());
        return Ok(());
    };

    debug!("Running '{:?}'", command);

    let repo = repo_from_path(&args.repo_path)?;

    validate_repo(&repo)?;

    let config = Config::load(&repo, args.config)?;

    match command {
        Command::Cache { command } => match command {
            CacheCommand::Create => {
                println!("Creating the cache");
                let cache_path = cache::create_cache()?;
                println!("Cache created at {}", cache_path.to_string_lossy());
                info!("Populating the cache for the repo after cache creation");
                cache::update_cache_for_repo(&repo)?
            }
            CacheCommand::Update => {
                println!("Updating the cache");
                cache::update_cache_for_repo(&repo)?;
                println!("Cache updated");
            }

            CacheCommand::Drop => {
                println!("Dropping the cache for the repo");
                if let Some(repo_path) = cache::drop_cache_for_repo(&repo)? {
                    println!("Dropped the cache for repo at '{:?}'", repo_path);
                } else {
                    println!(
                        "Cache for repo at '{:?}' does not exist, not doing a thing",
                        repo.path()
                    );
                }
            }

            CacheCommand::Nuke => {
                println!("Removing the whole cache");
                if cache::nuke_cache()? {
                    println!("Cache is no more. It ceased to be.");
                } else {
                    println!("Cache does not exist");
                }
            }

            CacheCommand::Show => {
                let cache = cache::show_cache()?;
                println!("Cached repos:");
                for (k, v) in cache.entries {
                    println!(
                        "- {}: timestamp: {}, hash: {}",
                        k.to_string_lossy(),
                        v.timestamp,
                        v.head_commit_hash
                    );
                }
            }
        },
        Command::Type { json } => {
            let output = commit_types::get_commit_types_from_repo_or_default(config)?;

            match json {
                true => json_print(output)?,
                false => default_print(output),
            }
        }
        Command::Scope { json } => {
            let output = commit_scopes::try_get_commit_scopes_from_repo(&repo, config)?
                .unwrap_or_else(Vec::new);

            match json {
                true => json_print(output)?,
                false => default_print(output),
            }
        }
    };

    Ok(())
}
