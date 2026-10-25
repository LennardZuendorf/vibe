//! The command line: clap for the shape, a hand-rolled leading-option scan for
//! `policy --vibe-dir` (mirrors `commands/policy.mjs`'s `parseLeadingOptions`).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use crate::cmd::{self, Outcome};
use crate::hook;
use crate::paths::{Env, FlowPaths};

const VIBE_DIR_FLAG: &str = "--vibe-dir";

/// The vibe flow tool: cursor, write policy and state machine.
#[derive(Parser, Debug)]
#[command(name = "vibe-flow", version, about, arg_required_else_help = true)]
struct Cli {
    /// Project root; skips discovery.
    #[arg(long, value_name = "DIR", global = true)]
    root: Option<PathBuf>,
    /// Shipped flow data home; skips discovery.
    #[arg(long, value_name = "DIR", global = true)]
    vibe_dir: Option<PathBuf>,
    /// Accepted for the engine's CLI shape; unused.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Read or write the flow cursor.
    State {
        #[command(subcommand)]
        command: StateCommand,
    },
    /// Query the write policy.
    Policy {
        #[command(subcommand)]
        command: PolicyCommand,
    },
    /// Run one hook event over the stdin payload (the platform's exit-code
    /// convention: 0 pass or degrade, 2 block).
    Hook {
        /// The event name, e.g. `pre-tool-use-guard` or `stop-gate`.
        event: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum StateCommand {
    /// Print the resolved cursor as pretty JSON.
    Get,
    /// Write the cursor: `set <flow.phase> [feature]`.
    Set {
        /// The target state, e.g. `feature.impl` or `idle`.
        target: Option<String>,
        /// The feature to attach; carried forward when omitted.
        feature: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum PolicyCommand {
    /// Print the verdict for writing <PATH> now (a refusal exits 2).
    Decide {
        /// The path to check.
        path: Option<String>,
        /// The state to check against; defaults to the cursor.
        state: Option<String>,
    },
    /// The same verdict as `decide`, but a refusal exits 1.
    Check {
        /// The path to check.
        path: Option<String>,
        /// The state to check against; defaults to the cursor.
        state: Option<String>,
    },
    /// Print the loaded rules.
    List,
    /// Print the `{{invariants}}` prose.
    Render,
}

/// Parses and runs one invocation. `argv` includes the program name.
pub fn run(cwd: &Path, argv: &[OsString], env: &dyn Env) -> (u8, Vec<u8>, Vec<u8>) {
    let args: Vec<OsString> = argv.iter().skip(1).cloned().collect();
    let (args, leading_vibe_dir) = presplit(&args);
    let program = argv
        .first()
        .cloned()
        .unwrap_or_else(|| OsString::from("vibe-flow"));
    let parsed = match Cli::try_parse_from(std::iter::once(program).chain(args)) {
        Ok(parsed) => parsed,
        Err(err) => {
            let code = u8::try_from(err.exit_code()).unwrap_or(2);
            let _ = err.print();
            return (code, Vec::new(), Vec::new());
        }
    };
    let _ = parsed.json;
    let vibe_dir = leading_vibe_dir
        .map(PathBuf::from)
        .or_else(|| parsed.vibe_dir.clone());
    let paths = FlowPaths::resolve(parsed.root.as_deref(), vibe_dir.as_deref(), cwd, env);
    let outcome = dispatch(&paths, parsed.command);
    (
        outcome.code,
        outcome.stdout.into_bytes(),
        outcome.stderr.into_bytes(),
    )
}

fn dispatch(paths: &FlowPaths, command: Command) -> Outcome {
    match command {
        Command::State { command } => match command {
            StateCommand::Get => cmd::state::run_get(paths),
            StateCommand::Set { target, feature } => {
                cmd::state::run_set(paths, target.as_deref(), feature.as_deref())
            }
        },
        Command::Policy { command } => match command {
            PolicyCommand::Decide { path, state } => {
                cmd::policy::run(paths, cmd::policy::Sub::Decide, &operands(path, state))
            }
            PolicyCommand::Check { path, state } => {
                cmd::policy::run(paths, cmd::policy::Sub::Check, &operands(path, state))
            }
            PolicyCommand::List => cmd::policy::run(paths, cmd::policy::Sub::List, &[]),
            PolicyCommand::Render => cmd::policy::run(paths, cmd::policy::Sub::Render, &[]),
        },
        Command::Hook { event } => {
            // The composition events and an absent name read no stdin: a
            // nameless or unknown hook from a terminal must not block on EOF.
            match event.as_deref() {
                Some(name @ (hook::GUARD | hook::STOP)) => {
                    hook::run(paths, name, &hook::read_stdin())
                }
                _ => Outcome::new(0, String::new(), String::new()),
            }
        }
    }
}

fn operands(path: Option<String>, state: Option<String>) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(path) = path {
        args.push(path);
    }
    if let Some(state) = state {
        args.push(state);
    }
    args
}

/// Pulls a leading `--vibe-dir <dir>` / `--vibe-dir=<dir>` off the arguments
/// that follow `policy`: leading-only, repeats allowed (last wins), and a flag
/// with no value is dropped rather than treated as an error.
fn parse_leading_options(args: &[OsString]) -> (Vec<OsString>, Option<String>) {
    let mut rest: Vec<OsString> = args.to_vec();
    let mut vibe_dir: Option<String> = None;
    while !rest.is_empty() {
        let head = rest[0].to_string_lossy().into_owned();
        if head == VIBE_DIR_FLAG {
            rest.remove(0);
            if !rest.is_empty() {
                let value = rest.remove(0).to_string_lossy().into_owned();
                if !value.is_empty() {
                    vibe_dir = Some(value);
                }
            }
            continue;
        }
        if let Some(value) = head.strip_prefix(&format!("{VIBE_DIR_FLAG}=")) {
            rest.remove(0);
            if !value.is_empty() {
                vibe_dir = Some(value.to_string());
            }
            continue;
        }
        break;
    }
    (rest, vibe_dir)
}

/// Splits the leading global flags from the subcommand and applies
/// [`parse_leading_options`] when that subcommand is `policy`.
fn presplit(args: &[OsString]) -> (Vec<OsString>, Option<String>) {
    let mut index = 0;
    while index < args.len() {
        let token = args[index].to_string_lossy().into_owned();
        if token == "--root" || token == VIBE_DIR_FLAG {
            index += 2;
            continue;
        }
        if token.starts_with("--root=") || token.starts_with(&format!("{VIBE_DIR_FLAG}=")) {
            index += 1;
            continue;
        }
        if token == "--json" {
            index += 1;
            continue;
        }
        break;
    }
    if index < args.len() && args[index].to_string_lossy() == "policy" {
        let (rest, vibe_dir) = parse_leading_options(&args[index + 1..]);
        let mut out: Vec<OsString> = args[..=index].to_vec();
        out.extend(rest);
        return (out, vibe_dir);
    }
    (args.to_vec(), None)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{parse_leading_options, presplit};
    use std::ffi::OsString;

    fn args(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    fn strings(items: &[OsString]) -> Vec<String> {
        items
            .iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn leading_options_take_the_last_value_and_stop_at_the_first_operand() {
        let (rest, dir) =
            parse_leading_options(&args(&["--vibe-dir", "/a", "--vibe-dir=/b", "decide", "p"]));
        assert_eq!(strings(&rest), ["decide", "p"]);
        assert_eq!(dir.as_deref(), Some("/b"));
    }

    #[test]
    fn a_leading_option_with_no_value_is_dropped() {
        let (rest, dir) = parse_leading_options(&args(&["--vibe-dir"]));
        assert!(rest.is_empty());
        assert_eq!(dir, None);
    }

    #[test]
    fn an_empty_leading_value_is_ignored() {
        let (rest, dir) = parse_leading_options(&args(&["--vibe-dir=", "list"]));
        assert_eq!(strings(&rest), ["list"]);
        assert_eq!(dir, None);
    }

    #[test]
    fn options_after_the_subcommand_are_left_alone() {
        let (rest, dir) = parse_leading_options(&args(&["decide", "--vibe-dir", "/a"]));
        assert_eq!(strings(&rest), ["decide", "--vibe-dir", "/a"]);
        assert_eq!(dir, None);
    }

    #[test]
    fn presplit_only_touches_the_policy_subcommand() {
        let (out, dir) = presplit(&args(&["state", "get"]));
        assert_eq!(strings(&out), ["state", "get"]);
        assert_eq!(dir, None);

        let (out, dir) = presplit(&args(&[
            "--root",
            "/r",
            "policy",
            "--vibe-dir",
            "/p",
            "list",
        ]));
        assert_eq!(strings(&out), ["--root", "/r", "policy", "list"]);
        assert_eq!(dir.as_deref(), Some("/p"));
    }

    #[test]
    fn presplit_does_not_mistake_a_value_for_the_subcommand() {
        let (out, dir) = presplit(&args(&["--root", "policy", "state", "get"]));
        assert_eq!(strings(&out), ["--root", "policy", "state", "get"]);
        assert_eq!(dir, None);
    }
}
