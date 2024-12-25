use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use log::debug;
use std::path::PathBuf;

use self::commit_types::get_default_commit_types;
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
    /// Delete the whole cache
    Nuke,
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

fn json_print<S>(output: Vec<PrintableEntity<S>>)
where
    S: serde::Serialize,
    std::string::String: std::convert::From<S>,
{
    println!("{}", serde_json::to_string(&output).unwrap())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();

    debug!("Launched with args: {:?}", args);

    // Handle no given command. This should be done first so nothing is really validated.
    let Some(command) = args.command else {
        debug!("Running in default mode, just printing the types");
        default_print(get_default_commit_types());
        return Ok(());
    };

    debug!("Running '{:?}'", command);

    let repo = repo_from_path(&args.repo_path)?;

    validate_repo(&repo)?;
    match command {
        Command::Cache { command } => match command {
            CacheCommand::Create => {
                cache::create_cache()?;
                debug!("Populating the cache for the repo after cache creation");
                cache::update_cache_for_repo(&repo)?
            }
            CacheCommand::Update => cache::update_cache_for_repo(&repo)?,

            CacheCommand::Drop => cache::drop_cache_for_repo(&repo)?,

            CacheCommand::Nuke => cache::nuke_cache()?,
        },
        Command::Type { json } => {
            let output = commit_types::get_commit_types_from_repo_or_default(&repo)?;

            match json {
                true => json_print(output),
                false => default_print(output),
            }
        }
        Command::Scope { json } => {
            let output =
                commit_scopes::try_get_commit_scopes_from_repo(&repo)?.unwrap_or_else(Vec::new);

            match json {
                true => json_print(output),
                false => default_print(output),
            }
        }
    };

    Ok(())
}
