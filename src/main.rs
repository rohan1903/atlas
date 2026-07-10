mod commands;
mod diagram;
mod graph;
mod highlight;
mod intelligence;
mod parse;
mod paths;
mod progress;
mod scan;
mod store;
mod style;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "atlas",
    version,
    about = "Repository intelligence for large codebases",
    long_about = "Atlas builds a structural map of a repository so you can understand architecture, important files, and flows — without writing code for you."
)]
struct Cli {
    /// Force color output (snippets and styled text)
    #[arg(long, global = true)]
    color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a repository and store intelligence under .atlas/
    Scan {
        /// Repository path to scan (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Show why files and directories were skipped
        #[arg(short, long)]
        verbose: bool,

        /// Print inventoried file paths (capped at 50; full list stays in inventory.json)
        #[arg(long)]
        list: bool,

        /// Delete existing .atlas/ and rebuild from scratch
        #[arg(long)]
        force: bool,
    },
    /// Show subsystems, entrypoints, and critical files
    Architecture {
        /// Repository that was previously scanned (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Show importance-ranked files
    TopFiles {
        /// Repository that was previously scanned (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Maximum number of files to show
        #[arg(long, default_value_t = commands::top_files::default_limit())]
        limit: usize,

        /// Include test files in the ranked list (excluded by default)
        #[arg(long)]
        include_tests: bool,

        /// Include docs, config, and deployment files (excluded by default)
        #[arg(long)]
        include_metadata: bool,
    },
    /// Trace an execution path for a function or feature name
    Flow {
        /// Function, route, or feature name to trace (e.g. login, core_init)
        name: String,

        /// Repository that was previously scanned
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Show the full call graph instead of a compressed primary path
        #[arg(long)]
        verbose: bool,
    },
    /// Show a recommended reading order for a subsystem topic
    Learn {
        /// Subsystem or topic name (e.g. auth, drivers/gpu/drm)
        topic: String,

        /// Repository that was previously scanned
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Explain a topic from graph evidence
    Explain {
        /// Subsystem or topic name (e.g. auth, orders)
        topic: String,

        /// Repository that was previously scanned
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

fn main() {
    highlight::init_terminal_colors();
    let cli = Cli::parse();
    if cli.color {
        highlight::set_force_color(true);
    }

    let result = match cli.command {
        Commands::Scan {
            path,
            verbose,
            list,
            force,
        } => scan::run(&path, verbose, list, force),
        Commands::Architecture { path } => {
            let repo = store::resolve_repo(&path);
            match repo {
                Ok(repo) => commands::architecture::run(&repo),
                Err(error) => Err(error),
            }
        }
        Commands::TopFiles {
            path,
            limit,
            include_tests,
            include_metadata,
        } => {
            let repo = store::resolve_repo(&path);
            match repo {
                Ok(repo) => commands::top_files::run(&repo, limit, include_tests, include_metadata),
                Err(error) => Err(error),
            }
        }
        Commands::Flow {
            path,
            name,
            verbose,
        } => {
            let repo = store::resolve_repo(&path);
            match repo {
                Ok(repo) => commands::flow::run(&repo, &name, verbose),
                Err(error) => Err(error),
            }
        }
        Commands::Learn { path, topic } => {
            let repo = store::resolve_repo(&path);
            match repo {
                Ok(repo) => commands::learn::run(&repo, &topic),
                Err(error) => Err(error),
            }
        }
        Commands::Explain { path, topic } => {
            let repo = store::resolve_repo(&path);
            match repo {
                Ok(repo) => commands::explain::run(&repo, &topic),
                Err(error) => Err(error),
            }
        }
    };

    if let Err(error) = result {
        eprintln!("{} {}", style::error("error:"), error);
        process::exit(1);
    }
}
