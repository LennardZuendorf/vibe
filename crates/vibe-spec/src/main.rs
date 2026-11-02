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
mod init;
mod layered;
mod lessons;
mod list_specs;
mod literal;
mod migrate;
mod openspec;
mod pattern;
mod promote;
mod query;
mod report;
mod root;
mod scan_merges;
mod setup;
mod superpowers;
mod text;
mod trace;
mod transcript;
mod validate;

/// The vibe spec tool.
#[derive(Parser)]
#[command(
    name = "vibe-spec",
    version,
    about,
    arg_required_else_help = true,
    after_help = "Environment:\n  VIBE_SPEC_ROOT  spec root (overridden by --root)\nExit: 0 ok, 1 findings or refused, 2 usage, 3 no spec root"
)]
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
    /// List spec files with type and scope.
    #[command(disable_help_flag = true)]
    ListSpecs {
        /// Ignored, as the oracle ignores them.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Print lessons whose tags match (`--format markdown|inject|json`; `--json` for Spec JSON v1).
    #[command(disable_help_flag = true)]
    LessonsFor {
        /// Tags, `--format <markdown|inject|json>`, `--tag <t>`, `--json`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Report pending merge blocks in feature tech docs.
    #[command(disable_help_flag = true)]
    ScanMerges {
        /// `[<feature>] [--format table|json|plain]`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Validate the spec tree (structure, links, frontmatter, strictness).
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
    /// Check that merged features left a wrap-up trail.
    ///
    /// `--pre-merge` also fails on superpowers artifacts left under
    /// docs/superpowers/.
    CheckDrift {
        /// Superpowers artifacts are errors (run before merging).
        #[arg(long)]
        pre_merge: bool,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// Merge a feature's durable facts into the root spec.
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
    /// Seed a layered spec root (README, AGENTS, the four hubs and lessons).
    ///
    /// Targets `--to`, else the discovered root, else `docs/spec`. Never
    /// overwrites a file (`SKIP:`); refuses a symlinked target path.
    Init {
        /// The directory to seed (relative to the current directory).
        #[arg(long, value_name = "DIR")]
        to: Option<PathBuf>,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// Install the pre-commit hook, OpenCode files and the AGENTS.md router block.
    ///
    /// Needs no spec root: with or without one it exits 0 on success.
    Setup {
        /// Install the git pre-commit hook.
        #[arg(long)]
        git_hooks: bool,
        /// Write the OpenCode agents and plugin shim.
        #[arg(long)]
        opencode: bool,
        /// Add or refresh the router block in AGENTS.md.
        #[arg(long)]
        agents_md: bool,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// Move the spec root and rewrite legacy-root literals.
    ///
    /// Plans, then moves (`git mv` for tracked files), rewrites, writes
    /// `.vibe/spec.json` `root`, checks that no literal is left and inserts
    /// allow-markers for the warnings the move leaves. Any failure restores
    /// the tree and the git index. `--dry-run` prints the plan; `--check` is
    /// the zero-literal sweep. Both write nothing.
    Migrate {
        /// The new spec root, relative to the current directory.
        #[arg(long, value_name = "DIR")]
        to: PathBuf,
        /// Also restructure into the layered layout.
        #[arg(long, value_enum)]
        layout: Option<Layout>,
        /// Print the plan and write nothing.
        #[arg(long, conflicts_with = "check")]
        dry_run: bool,
        /// Exit 1 when a literal remains; write nothing.
        #[arg(long)]
        check: bool,
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

/// `migrate --layout`.
#[derive(Clone, Copy, clap::ValueEnum)]
enum Layout {
    /// The layered spec layout.
    Layered,
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
            if matches!(cli.command, Command::Hook { .. }) {
                // Hooks fail open: a usage exit would block the session.
                eprintln!("vibe-spec: cannot read the current directory: {e} — hook skipped");
                return ExitCode::SUCCESS;
            }
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
                let mut stdin = Vec::new();
                let _ = std::io::Read::read_to_end(&mut std::io::stdin(), &mut stdin);
                let harness = hook::Harness::detect(|k| std::env::var_os(k).is_some());
                let (code, out) = hook::session_start(&stdin, &cwd, harness);
                emit(&out);
                code
            }
        };
        return ExitCode::from(code);
    }
    let env_root = std::env::var_os("VIBE_SPEC_ROOT");
    let found = root::discover(&cwd, cli.root.as_deref(), env_root.as_deref());
    if let Ok(root) = &found
        && let Some(warning) = root::both_roots_warning(root)
    {
        eprintln!("{warning}");
    }
    let (code, out) = match cli.command {
        Command::Root { json } => root::command(&found, json),
        Command::ListSpecs { args: _ } => list_specs::run(&found, &raw_tail(std::env::args_os())),
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
        Command::Init { to, json } => init::run(&found, &cwd, to.as_deref(), json),
        Command::Setup {
            git_hooks,
            opencode,
            agents_md,
            json,
        } => setup::run(
            &found,
            &cwd,
            &setup::Options {
                git_hooks,
                opencode,
                agents_md,
                json,
            },
        ),
        Command::Migrate {
            to,
            layout,
            dry_run,
            check,
            json,
        } => migrate::run(
            &found,
            &cwd,
            &migrate::Options {
                to,
                layered: layout.is_some(),
                dry_run,
                check,
                json,
            },
        ),
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

/// Whether a raw-tail command was asked for help: a `-h` or `--help` token
/// that is not the value of `--format`, `--tag` or `--target`.
pub(crate) fn wants_help(args: &[String]) -> bool {
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        match arg.as_str() {
            "--format" | "--tag" | "--target" => i += 1,
            "-h" | "--help" => return true,
            _ => {}
        }
        i += 1;
    }
    false
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
    fn wants_help_skips_option_values() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(super::wants_help(&v(&["foo", "--help"])));
        assert!(super::wants_help(&v(&["-h"])));
        assert!(!super::wants_help(&v(&["--tag", "-h"])));
        assert!(!super::wants_help(&v(&["--format", "--help"])));
        assert!(!super::wants_help(&v(&["--target", "-h"])));
        assert!(super::wants_help(&v(&["--tag", "x", "-h"])));
        assert!(!super::wants_help(&[]));
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
