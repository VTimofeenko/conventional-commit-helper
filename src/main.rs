use std::path::PathBuf;
use clap::{ArgAction, Parser, ValueEnum};
use log::debug;

use self::commit_types::get_default_commit_types;

mod commit_types;
mod utils;

#[derive(ValueEnum, Clone, Debug)]
enum Mode {
    Type,
    Scope,
}

/// Tiny helper for conventional commits (https://www.conventionalcommits.org).
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /////Mode in which the program runs
    #[clap(value_enum, default_value=None)]
    mode: Option<Mode>,

    /// Print output in JSON format
    #[arg(long)]
    json: bool,

    /// Path to the non-bare git repository.
    #[arg(long, default_value = ".")]
    repo_path: PathBuf,

    #[arg(long, action=ArgAction::SetTrue)]
    debug: bool,
    // /// Path to the file containing conventional commit types for the repository.
    // ///
    // /// Can be specified as relative to the repo workdir root (default value)
    // #[arg(long, default_value = ".dev/commit-types.json")]
    // commit_types_file: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.debug {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .format_timestamp_millis() // ms needed for perf troubleshooting
            .init();

        debug!("Launched with args: {:?}", args);
    }

    let output = match args.mode {
        Some(x) => match x {
            Mode::Type => {
                commit_types::get_commit_types_from_repo_or_default(args.repo_path)?
            }
            Mode::Scope => todo!(),
        },
        None => {
            debug!("No modes passed as an arg, running default action");
            get_default_commit_types()
        }
    };

    match args.json {
        true => println!("{}", serde_json::to_string(&output).unwrap()),
        false => output.iter().for_each(|x| println!("{}", x)),
    }
    Ok(())
}
