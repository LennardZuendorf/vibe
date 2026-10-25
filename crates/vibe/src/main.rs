//! `vibe`: the v0.5 CLI front door.
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use vibe::cmd;
use vibe::paths::{Harness, Paths};
use vibe::store::{import, init};
use vibe_core::dirs::Env;
use vibe_core::exit;

/// One global store, projected into every agent harness.
#[derive(Parser)]
#[command(name = "vibe", version, about, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage the global store.
    Store {
        #[command(subcommand)]
        action: StoreAction,
    },
    /// Project the store into every harness.
    Sync {
        /// Harnesses to sync, comma separated.
        #[arg(long, value_delimiter = ',')]
        harness: Vec<String>,
        /// Show the plan without writing.
        #[arg(long)]
        dry_run: bool,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Run a harness hook.
    Hook {
        #[command(subcommand)]
        event: HookEvent,
    },
    /// Wire vibe into the harnesses.
    Install {
        /// Harnesses to install into, comma separated; forces undetected ones too.
        #[arg(long, value_delimiter = ',')]
        harness: Vec<String>,
        /// Directory holding vibe's own skills.
        #[arg(long, value_name = "DIR")]
        toolkit: Option<PathBuf>,
        /// Show the plan without writing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove everything vibe created.
    Uninstall {
        /// Show the plan without writing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Check the wiring.
    Doctor,
    /// The spec tool.
    Spec {
        /// Arguments passed to the spec tool.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum StoreAction {
    /// Create the store.
    Init {
        /// Seed the root instructions from this file.
        #[arg(long, value_name = "FILE")]
        instructions: Option<PathBuf>,
    },
    /// Import an MCP list into the store.
    ImportMcp {
        /// The file to import.
        file: PathBuf,
        /// Keep literal secrets instead of placeholders.
        #[arg(long)]
        keep_literals: bool,
    },
    /// Show the store's state.
    Status {
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum HookEvent {
    /// Re-project the store at session start.
    SessionStart {
        /// The harness that started the session.
        #[arg(long)]
        harness: Option<String>,
    },
}

fn refuse(msg: &str) -> ExitCode {
    eprintln!("vibe: {msg}");
    ExitCode::from(exit::FINDINGS)
}

fn store_command(action: StoreAction) -> ExitCode {
    let paths = match Paths::from_env(&Env::from_process()) {
        Ok(p) => p,
        Err(e) => return refuse(&e.to_string()),
    };
    match action {
        StoreAction::Init { instructions } => match init::init(&paths, instructions.as_deref()) {
            Ok(report) => {
                for e in report.entries {
                    let word = if e.created { "created" } else { "exists" };
                    println!("{word} {}", e.path.display());
                }
                ExitCode::SUCCESS
            }
            Err(e) => refuse(&e),
        },
        StoreAction::ImportMcp {
            file,
            keep_literals,
        } => match import::import(&file, &paths, keep_literals) {
            Ok(report) => {
                for name in &report.added {
                    println!("added {name}");
                }
                for name in &report.kept_existing {
                    println!("kept {name} (already in store)");
                }
                for var in &report.vars {
                    println!("export {var}");
                }
                for w in &report.warnings {
                    eprintln!("warning: {w}");
                }
                ExitCode::SUCCESS
            }
            Err(e) => refuse(&e),
        },
        StoreAction::Status { json } => code(cmd::status::status(
            &paths,
            json,
            &mut io::stdout(),
            &mut io::stderr(),
        )),
    }
}

fn code(c: u8) -> ExitCode {
    ExitCode::from(c)
}

fn parse_harnesses(ids: &[String]) -> Result<Option<Vec<Harness>>, String> {
    if ids.is_empty() {
        return Ok(None);
    }
    ids.iter()
        .map(|id| Harness::parse(id).ok_or_else(|| format!("unknown harness `{id}`")))
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn usage(msg: &str) -> ExitCode {
    eprintln!("vibe: {msg}");
    ExitCode::from(exit::USAGE)
}

fn sync_command(harness: &[String], dry_run: bool, json: bool) -> ExitCode {
    let harnesses = match parse_harnesses(harness) {
        Ok(h) => h,
        Err(e) => return usage(&e),
    };
    let paths = match Paths::from_env(&Env::from_process()) {
        Ok(p) => p,
        Err(e) => return refuse(&e.to_string()),
    };
    code(cmd::sync::sync(
        &paths,
        harnesses,
        dry_run,
        json,
        "claude",
        &mut io::stdout(),
        &mut io::stderr(),
    ))
}

fn install_command(harness: &[String], toolkit: Option<PathBuf>, dry_run: bool) -> ExitCode {
    let harnesses = match parse_harnesses(harness) {
        Ok(h) => h,
        Err(e) => return usage(&e),
    };
    let paths = match Paths::from_env(&Env::from_process()) {
        Ok(p) => p,
        Err(e) => return refuse(&e.to_string()),
    };
    let opts = cmd::install::InstallOpts {
        harnesses,
        toolkit,
        dry_run,
    };
    code(cmd::install::install(
        &paths,
        opts,
        "claude",
        &mut io::stdout(),
        &mut io::stderr(),
    ))
}

fn uninstall_command(dry_run: bool) -> ExitCode {
    let paths = match Paths::from_env(&Env::from_process()) {
        Ok(p) => p,
        Err(e) => return refuse(&e.to_string()),
    };
    code(cmd::uninstall::uninstall(
        &paths,
        dry_run,
        "claude",
        &mut io::stdout(),
        &mut io::stderr(),
    ))
}

fn doctor_command() -> ExitCode {
    let paths = match Paths::from_env(&Env::from_process()) {
        Ok(p) => p,
        Err(e) => return refuse(&e.to_string()),
    };
    code(cmd::doctor::doctor(
        &paths,
        &mut io::stdout(),
        &mut io::stderr(),
    ))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Store { action } => store_command(action),
        Command::Sync {
            harness,
            dry_run,
            json,
        } => sync_command(&harness, dry_run, json),
        Command::Hook {
            event: HookEvent::SessionStart { harness },
        } => code(cmd::hook::run_from_env(
            harness.as_deref().and_then(Harness::parse),
            &mut io::stdout(),
            &mut io::stderr(),
        )),
        Command::Install {
            harness,
            toolkit,
            dry_run,
        } => install_command(&harness, toolkit, dry_run),
        Command::Uninstall { dry_run } => uninstall_command(dry_run),
        Command::Doctor => doctor_command(),
        Command::Spec { args } => {
            let args: Vec<OsString> = args.into_iter().map(OsString::from).collect();
            code(cmd::spec::spec(&args, &Env::from_process()))
        }
    }
}
