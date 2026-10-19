//! `vibe-spec`: the spec tool. F16 ports the read-only spec scripts and adds
//! root discovery and the Spec JSON v1 queries; F17 adds the check registry,
//! strictness, the F17 validators and `trace`.
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use vibe_core::exit;

/// Concatenates byte-slice-like values (`&str`, `&[u8]`, `Vec<u8>`, `String`) into one `Vec<u8>`.
macro_rules! cat {
    ($($part:expr),* $(,)?) => {{
        let mut v: Vec<u8> = Vec::new();
        $(v.extend_from_slice(AsRef::<[u8]>::as_ref(&$part));)*
        v
    }};
}

mod adherence;
mod check;
mod content;
mod delta;
mod drift;
mod frontmatter;
mod git;
mod hook;
mod layered;
mod lessons;
mod list_specs;
mod openspec;
mod pattern;
mod promote;
mod query;
mod report;
mod root;
mod scan_merges;
mod superpowers;
mod text;
mod trace;
mod transcript;
mod validate;

/// The vibe spec tool.
#[derive(Parser)]
#[command(name = "vibe-spec", version, about, arg_required_else_help = true)]
struct Cli {
    /// Spec root directory; skips discovery. Goes before the command.
    #[arg(long, value_name = "DIR")]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the spec root and how it was found.
    Root {
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// List spec files (port of list-specs.sh).
    #[command(disable_help_flag = true)]
    ListSpecs {
        /// Ignored, as the oracle ignores them.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Lessons matching tags (port of lessons-for.sh; `--json` for Spec JSON v1).
    #[command(disable_help_flag = true)]
    LessonsFor {
        /// Tags, `--format <markdown|inject|json>`, `--tag <t>`, `--json`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Report `<!-- merge -->` blocks in feature tech docs (port of scan-merges.sh).
    #[command(disable_help_flag = true)]
    ScanMerges {
        /// `[<feature>] [--format table|json|plain]`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Validate the spec tree (port of validate.sh, plus F17's checks).
    Validate {
        /// Promote every non-off check to error; refuse a run that examined nothing.
        #[arg(long)]
        strict: bool,
        /// Output format.
        #[arg(long, value_enum, default_value = "text")]
        format: validate::Format,
        /// Same as `--format json`.
        #[arg(long)]
        json: bool,
        /// Keep the findings of files changed in git (or since `.vibe/spec-session`), and of the tree.
        #[arg(long, conflicts_with = "file")]
        changed: bool,
        /// Keep only this file's findings.
        #[arg(long)]
        file: Option<String>,
        /// The hook event for `--format hook`: PostToolUse or Stop.
        #[arg(long)]
        event: Option<String>,
    },
    /// A feature's plan rows (Spec JSON v1 `plan` with --json).
    Plan {
        /// Feature name (lowercase-kebab).
        #[arg(long)]
        feature: String,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// A feature's layer and docs (Spec JSON v1 `feature` with --json).
    Feature {
        /// Feature name (lowercase-kebab).
        name: String,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// Compound drift gate (port of check-drift.sh); `--pre-merge` also fails
    /// on superpowers artifacts left under docs/superpowers/.
    CheckDrift {
        /// Superpowers artifacts are errors (run before merging).
        #[arg(long)]
        pre_merge: bool,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// Promote a feature (port of promote.sh; requirement deltas; superpowers).
    #[command(disable_help_flag = true)]
    Promote {
        /// `<feature> [--dry-run] [--target <file>] [--done] [--base <ref>] [--json]`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Write the spec tree as an OpenSpec directory (`<DIR>/openspec/`).
    Export {
        /// Output directory; `openspec/` is created inside it.
        #[arg(long, value_name = "DIR")]
        openspec: PathBuf,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// Import OpenSpec changes as fallback feature folders.
    Import {
        /// Directory holding `openspec/`.
        #[arg(long, value_name = "DIR")]
        openspec: PathBuf,
        /// Import only this change.
        #[arg(long)]
        feature: Option<String>,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// Units cited by commits and tests/, uncited units, orphan citations.
    Trace {
        /// Only commits after this revision (`<ref>..HEAD`).
        #[arg(long, value_name = "REF", allow_hyphen_values = true)]
        since: Option<String>,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// Hook entry points; each reads the hook payload on stdin.
    Hook {
        /// The hook event.
        #[arg(value_enum)]
        event: HookEvent,
    },
}

/// A hook event handled by `vibe-spec hook`.
#[derive(Clone, Copy, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
enum HookEvent {
    /// Prints the using-spec router.
    SessionStart,
    /// The PreToolUse tooth.
    Guard,
    /// Post-edit and stop validation.
    Validate,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("vibe-spec: cannot read the current directory: {e}");
            return ExitCode::from(exit::USAGE);
        }
    };
    if std::env::var_os("SPEC_DIR").is_some() {
        eprintln!("vibe-spec: SPEC_DIR is ignored; set VIBE_SPEC_ROOT instead");
    }
    if let Command::Hook { event } = &cli.command {
        let code = match event {
            HookEvent::Guard => {
                let mut stdin = Vec::new();
                let _ = std::io::Read::read_to_end(&mut std::io::stdin(), &mut stdin);
                hook::guard(&stdin).0
            }
            HookEvent::Validate => {
                let mut stdin = Vec::new();
                let _ = std::io::Read::read_to_end(&mut std::io::stdin(), &mut stdin);
                let (code, out) = hook::validate(&stdin, &cwd);
                emit(&out);
                code
            }
            HookEvent::SessionStart => {
                eprintln!("vibe-spec hook: this event is not implemented yet — allowing");
                exit::OK
            }
        };
        return ExitCode::from(code);
    }
    let env_root = std::env::var_os("VIBE_SPEC_ROOT");
    let found = root::discover(&cwd, cli.root.as_deref(), env_root.as_deref());
    let (code, out) = match cli.command {
        Command::Root { json } => root::command(&found, json),
        Command::ListSpecs { args: _ } => list_specs::run(&found),
        Command::LessonsFor { args: _ } => lessons::run(&found, &raw_tail(std::env::args_os())),
        Command::ScanMerges { args: _ } => {
            scan_merges::run(&found, &cwd, &raw_tail(std::env::args_os()))
        }
        Command::Validate {
            strict,
            format,
            json,
            changed,
            file,
            event,
        } => validate::run(
            &found,
            &cwd,
            &validate::Options {
                strict,
                format: if json { validate::Format::Json } else { format },
                changed,
                file,
                event,
            },
        ),
        Command::Plan { feature, json } => query::plan(&found, &feature, json),
        Command::Feature { name, json } => query::feature(&found, &name, json),
        Command::CheckDrift { pre_merge, json } => {
            drift::run(&found, &cwd, &drift::Options { pre_merge, json })
        }
        Command::Promote { args: _ } => promote::run(&found, &cwd, &raw_tail(std::env::args_os())),
        Command::Export {
            openspec: dir,
            json,
        } => openspec::export(&found, &cwd, &dir, json),
        Command::Import {
            openspec: dir,
            feature,
            json,
        } => openspec::import(&found, &cwd, &dir, feature.as_deref(), json),
        Command::Trace { since, json } => trace::run(&found, since.as_deref(), json),
        Command::Hook { .. } => (exit::OK, Vec::new()),
    };
    emit(&out);
    ExitCode::from(code)
}

/// The argv after the subcommand token, exactly as typed. clap strips a
/// leading `--` from trailing arguments, but the legacy commands must see it
/// (the oracles reject it as `Unknown option`). Every legacy subcommand
/// reads its arguments through this; `validate` no longer does (clap parses
/// its flags). Non-UTF-8 arguments are read lossily,
/// never a panic.
fn raw_tail(argv: impl IntoIterator<Item = OsString>) -> Vec<String> {
    let mut it = argv.into_iter().skip(1);
    match it.next() {
        Some(first) if first == "--root" => {
            it.next();
        }
        Some(first) if first.to_string_lossy().starts_with("--root=") => {}
        Some(_) => return it.map(|a| a.to_string_lossy().into_owned()).collect(),
        None => return Vec::new(),
    }
    it.next();
    it.map(|a| a.to_string_lossy().into_owned()).collect()
}

/// Writes `bytes` to stdout. A closed pipe is not an error.
fn emit(bytes: &[u8]) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(bytes);
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::raw_tail;
    use std::ffi::OsString;

    fn tail(argv: &[&str]) -> Vec<String> {
        raw_tail(argv.iter().map(OsString::from))
    }

    #[cfg(unix)]
    #[test]
    fn raw_tail_survives_non_utf8_arguments() {
        use std::os::unix::ffi::OsStringExt;
        let bad = OsString::from_vec(vec![0xff, b'a']);
        let argv = vec![
            OsString::from("vibe-spec"),
            OsString::from("--root"),
            bad.clone(),
            OsString::from("scan-merges"),
            bad,
        ];
        assert_eq!(raw_tail(argv), ["\u{fffd}a"]);
    }

    #[test]
    fn raw_tail_keeps_a_leading_double_dash() {
        assert_eq!(
            tail(&["vibe-spec", "--root", "x", "scan-merges", "--", "auth"]),
            ["--", "auth"]
        );
        assert_eq!(
            tail(&["vibe-spec", "--root=x", "lessons-for", "--"]),
            ["--"]
        );
        assert_eq!(tail(&["vibe-spec", "lessons-for", "--", "a"]), ["--", "a"]);
        assert!(tail(&["vibe-spec", "scan-merges"]).is_empty());
        assert!(tail(&["vibe-spec"]).is_empty());
    }
}
