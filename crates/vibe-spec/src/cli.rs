//! Compatibility argv parser and formatter.
use crate::{
    drift, hook, init, lessons, list_specs, migrate, openspec, promote, query, root, scan_merges,
    setup, trace, validate,
};
use clap::{Parser, Subcommand};
use std::ffi::OsString;
use std::path::PathBuf;
use vibe_core::exit;

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
    /// Select registered context for existing spec commands.
    #[arg(long)]
    project: Option<crate::ProjectId>,
    #[arg(long, conflicts_with = "branch")]
    checkout: Option<PathBuf>,
    #[arg(long)]
    branch: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Serve local spec tools over MCP stdio.
    Mcp {
        #[arg(long, required = true)]
        stdio: bool,
    },
    /// Preview or apply an explicit private-spec three-way merge.
    Reconcile {
        #[command(subcommand)]
        command: Option<ReconcileCommand>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        into: Option<String>,
        #[arg(long)]
        dry_run: bool,
        /// Apply the exact JSON plan returned by --dry-run; '-' reads stdin.
        #[arg(long)]
        input: Option<PathBuf>,
        #[arg(long)]
        expect: Option<String>,
        #[command(flatten)]
        selector: ManagementSelector,
    },
    /// Read and edit managed Markdown documents.
    Document {
        #[command(subcommand)]
        command: DocumentCommand,
    },
    /// Import raw global document edits into private spec history.
    Snapshot {
        #[arg(long)]
        expect: String,
        #[command(flatten)]
        selector: ManagementSelector,
    },
    /// Register or inspect spec projects without installing the workflow harness.
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Resolve a project, code checkout and spec branch.
    Context {
        #[command(flatten)]
        selector: ManagementSelector,
    },
    /// Initialize a stored spec branch from an explicit spec base.
    Branch {
        #[command(subcommand)]
        command: BranchCommand,
    },
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
        #[arg(long)]
        spec_only: bool,
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
        #[arg(long)]
        spec_only: bool,
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

/// Run standalone-compatible argv (including argv[0]) using only explicit inputs.
pub fn run_cli(
    argv: &[OsString],
    runtime: &crate::Runtime,
    input: &mut dyn std::io::Read,
) -> crate::CommandOutput {
    match parse(argv) {
        Ok(cli) => execute(cli, argv, runtime, input),
        Err(output) => output,
    }
}

#[derive(Clone, clap::Args)]
struct ManagementSelector {
    #[arg(long)]
    project: Option<crate::ProjectId>,
    #[arg(long)]
    checkout: Option<PathBuf>,
    #[arg(long)]
    branch: Option<String>,
}
impl ManagementSelector {
    fn context(&self) -> crate::ContextSelector {
        crate::ContextSelector {
            project: self.project.clone(),
            checkout: self.checkout.clone(),
            branch: self.branch.clone(),
        }
    }
}
#[derive(Subcommand)]
enum DocumentCommand {
    List {
        #[command(flatten)]
        selector: ManagementSelector,
    },
    Read {
        path: String,
        #[command(flatten)]
        selector: ManagementSelector,
    },
    Write {
        path: String,
        /// Read replacement UTF-8 Markdown from this file; omit to read stdin.
        #[arg(long, value_name = "FILE")]
        input: Option<PathBuf>,
        #[arg(long)]
        expect: String,
        #[command(flatten)]
        selector: ManagementSelector,
    },
    Remove {
        path: String,
        #[arg(long)]
        expect: String,
        #[command(flatten)]
        selector: ManagementSelector,
    },
}
#[derive(Subcommand)]
enum ProjectCommand {
    Init {
        #[arg(long, value_parser=["repo-first","global-first"])]
        mode: String,
        #[arg(long)]
        name: Option<String>,
        #[command(flatten)]
        selector: ManagementSelector,
        /// Exact registry revision; omitted values are captured once before the API call.
        #[arg(long)]
        expect: Option<String>,
    },
    Attach {
        #[arg(long)]
        project: crate::ProjectId,
        #[arg(long)]
        checkout: Option<PathBuf>,
        #[arg(long)]
        main: bool,
        #[arg(long)]
        expect: Option<String>,
    },
    List,
    Show {
        #[command(flatten)]
        selector: ManagementSelector,
    },
}
#[derive(Subcommand)]
enum BranchCommand {
    Init {
        name: String,
        #[arg(long)]
        from: String,
        #[command(flatten)]
        selector: ManagementSelector,
        #[arg(long)]
        expect: Option<String>,
        #[arg(long)]
        expect_project: Option<String>,
    },
}
#[derive(Subcommand)]
enum ReconcileCommand {
    Status {
        record: crate::RecordId,
        #[command(flatten)]
        selector: ManagementSelector,
    },
    Read {
        record: crate::RecordId,
        path: String,
        #[command(flatten)]
        selector: ManagementSelector,
    },
    Resolve {
        record: crate::RecordId,
        #[arg(long)]
        expect: String,
        #[arg(long)]
        expect_resolution: String,
        #[arg(long)]
        input: Option<PathBuf>,
        #[command(flatten)]
        selector: ManagementSelector,
    },
}

fn is_management(command: &Command) -> bool {
    matches!(
        command,
        Command::Reconcile { .. }
            | Command::Project { .. }
            | Command::Branch { .. }
            | Command::Context { .. }
            | Command::Document { .. }
            | Command::Snapshot { .. }
    )
}
fn management(
    command: &Command,
    runtime: &crate::Runtime,
    stdin: &mut dyn std::io::Read,
) -> Option<Result<crate::SpecResponse, crate::SpecError>> {
    if !is_management(command) {
        return None;
    }
    Some((|| {
        let api = crate::SpecApi::new(runtime.clone())?;
        let registry_revision = |expected: &Option<String>| -> Result<String, crate::SpecError> {
            match expected {
                Some(value) => Ok(value.clone()),
                None => Ok(crate::registry::load(runtime)?.revision),
            }
        };
        let request = match command {
            Command::Reconcile {
                command,
                from,
                into,
                dry_run,
                input,
                expect,
                selector,
            } => {
                if let Some(command) = command {
                    if from.is_some()
                        || into.is_some()
                        || *dry_run
                        || expect.is_some()
                        || input.is_some()
                        || selector.context() != crate::ContextSelector::default()
                    {
                        return Err(crate::SpecError::new(
                            "invalid_request",
                            "record operations accept only their own selectors and arguments",
                        ));
                    }
                    let selected = match command {
                        ReconcileCommand::Status { selector, .. }
                        | ReconcileCommand::Read { selector, .. }
                        | ReconcileCommand::Resolve { selector, .. } => selector.context(),
                    };
                    let registry = crate::registry::load(runtime)?;
                    let project = crate::storage::selected(&api, &registry, &selected)?
                        .id
                        .clone();
                    if selected.checkout.is_some() || selected.branch.is_some() {
                        let record_id = match command {
                            ReconcileCommand::Status { record, .. }
                            | ReconcileCommand::Read { record, .. }
                            | ReconcileCommand::Resolve { record, .. } => record,
                        };
                        let record = api.reconciliation_status(&project, record_id)?;
                        if selected
                            .branch
                            .as_deref()
                            .map(crate::context::code_ref)
                            .transpose()?
                            .as_ref()
                            .is_some_and(|branch| branch != &record.plan.target_ref)
                        {
                            return Err(crate::SpecError::new(
                                "context_mismatch",
                                "record selector differs from its captured target branch",
                            ));
                        }
                        if let Some(checkout) = &selected.checkout {
                            let path = std::fs::canonicalize(runtime.cwd.join(checkout))
                                .map_err(crate::documents::io)?;
                            let clean = crate::git::context_runtime(runtime);
                            let cx = crate::api::Execution::new(&clean);
                            let checkout = crate::context::checkout_top(&cx, &path)?;
                            if checkout.is_none() || checkout != record.plan.target_checkout {
                                return Err(crate::SpecError::new(
                                    "context_mismatch",
                                    "record selector differs from its captured target checkout",
                                ));
                            }
                        }
                    }
                    match command {
                        ReconcileCommand::Status { record, .. } => {
                            crate::SpecRequest::ReconciliationStatus {
                                project,
                                record_id: record.clone(),
                            }
                        }
                        ReconcileCommand::Read { record, path, .. } => {
                            crate::SpecRequest::ReconciliationRead {
                                project,
                                record_id: record.clone(),
                                path: path.clone(),
                            }
                        }
                        ReconcileCommand::Resolve {
                            record,
                            expect,
                            expect_resolution,
                            input,
                            ..
                        } => {
                            let mut bytes = Vec::new();
                            if let Some(input) =
                                input.as_ref().filter(|path| path.as_os_str() != "-")
                            {
                                bytes = std::fs::read(runtime.cwd.join(input))
                                    .map_err(crate::documents::io)?;
                            } else {
                                stdin
                                    .read_to_end(&mut bytes)
                                    .map_err(crate::documents::io)?;
                            }
                            let bundle: serde_json::Value = serde_json::from_slice(&bytes)
                                .map_err(|_| {
                                    crate::SpecError::new(
                                        "invalid_resolution",
                                        "resolution input must be a JSON document bundle",
                                    )
                                })?;
                            crate::SpecRequest::ReconciliationResolve(crate::ResolveRequest {
                                project,
                                record_id: record.clone(),
                                expected_target_revision: expect.clone(),
                                expected_resolution_revision: expect_resolution.clone(),
                                files: crate::reconcile::resolution_files(&bundle)?,
                            })
                        }
                    }
                } else {
                    if let Some(input) = input {
                        if *dry_run
                            || from.is_some()
                            || into.is_some()
                            || selector.context() != crate::ContextSelector::default()
                        {
                            return Err(crate::SpecError::new(
                                "invalid_request",
                                "exact plan input cannot be combined with preview, source, target or selector overrides",
                            ));
                        }
                        let expected_revision = expect.clone().ok_or_else(|| {
                            crate::SpecError::new(
                                "expected_revision_required",
                                "reconciliation apply requires --expect",
                            )
                        })?;
                        let mut bytes = Vec::new();
                        if input.as_os_str() == "-" {
                            stdin
                                .read_to_end(&mut bytes)
                                .map_err(crate::documents::io)?;
                        } else {
                            bytes = std::fs::read(runtime.cwd.join(input))
                                .map_err(crate::documents::io)?;
                        }
                        let value = serde_json::from_slice(&bytes).map_err(|_| {
                            crate::SpecError::new(
                                "invalid_reconciliation",
                                "expected a JSON reconciliation plan",
                            )
                        })?;
                        return api.execute(crate::SpecRequest::ReconcileApply {
                            plan: crate::ReconcilePlan::from_value(&value)?,
                            expected_revision,
                        });
                    }
                    let source = from.clone().ok_or_else(|| {
                        crate::SpecError::new("invalid_request", "reconciliation requires --from")
                    })?;
                    let target = into.clone().ok_or_else(|| {
                        crate::SpecError::new("invalid_request", "reconciliation requires --into")
                    })?;
                    if !dry_run && expect.is_none() {
                        return Err(crate::SpecError::new(
                            "expected_revision_required",
                            "reconciliation apply requires --expect",
                        ));
                    }
                    let plan = api.plan_reconciliation(crate::ReconcileRequest {
                        selector: selector.context(),
                        source,
                        target,
                    })?;
                    if *dry_run {
                        return Ok(crate::SpecResponse {
                            context: None,
                            data: plan.value(),
                        });
                    }
                    crate::SpecRequest::ReconcileApply {
                        plan,
                        expected_revision: expect.clone().ok_or_else(|| {
                            crate::SpecError::new(
                                "expected_revision_required",
                                "reconciliation apply requires --expect",
                            )
                        })?,
                    }
                }
            }

            Command::Document { command } => match command {
                DocumentCommand::List { selector } => {
                    crate::SpecRequest::Documents(selector.context())
                }
                DocumentCommand::Read { selector, path } => crate::SpecRequest::ReadDocument {
                    selector: selector.context(),
                    path: path.clone(),
                },
                DocumentCommand::Write {
                    selector,
                    path,
                    input,
                    expect,
                } => {
                    let mut bytes = Vec::new();
                    if let Some(input) = input {
                        bytes =
                            std::fs::read(runtime.cwd.join(input)).map_err(crate::documents::io)?;
                    } else {
                        stdin
                            .read_to_end(&mut bytes)
                            .map_err(crate::documents::io)?;
                    }
                    let content = String::from_utf8(bytes).map_err(|_| {
                        crate::SpecError::new(
                            "invalid_document",
                            "replacement Markdown must be UTF-8",
                        )
                    })?;
                    crate::SpecRequest::WriteDocument(crate::WriteDocumentRequest {
                        selector: selector.context(),
                        path: path.clone(),
                        content,
                        expected_revision: expect.clone(),
                    })
                }
                DocumentCommand::Remove {
                    selector,
                    path,
                    expect,
                } => crate::SpecRequest::RemoveDocument(crate::RemoveDocumentRequest {
                    selector: selector.context(),
                    path: path.clone(),
                    expected_revision: expect.clone(),
                }),
            },
            Command::Snapshot { selector, expect } => {
                crate::SpecRequest::Snapshot(crate::SnapshotRequest {
                    selector: selector.context(),
                    expected_revision: expect.clone(),
                })
            }
            Command::Project {
                command: ProjectCommand::List,
            } => crate::SpecRequest::Projects,
            Command::Project {
                command: ProjectCommand::Show { selector },
            } => crate::SpecRequest::ProjectShow {
                selector: selector.context(),
            },
            Command::Project {
                command:
                    ProjectCommand::Init {
                        mode,
                        name,
                        selector,
                        expect,
                    },
            } => crate::SpecRequest::ProjectInit(crate::InitProjectRequest {
                selector: selector.context(),
                mode: if mode == "global-first" {
                    crate::StorageMode::GlobalFirst
                } else {
                    crate::StorageMode::RepoFirst
                },
                name: name.clone(),
                expected_revision: registry_revision(expect)?,
            }),
            Command::Project {
                command:
                    ProjectCommand::Attach {
                        project,
                        checkout,
                        main,
                        expect,
                    },
            } => crate::SpecRequest::ProjectAttach(crate::AttachProjectRequest {
                project: project.clone(),
                checkout: checkout.clone(),
                main: *main,
                expected_revision: registry_revision(expect)?,
            }),
            Command::Context { selector } => crate::SpecRequest::Context(selector.context()),
            Command::Branch {
                command:
                    BranchCommand::Init {
                        name,
                        from,
                        selector,
                        expect,
                        expect_project,
                    },
            } => {
                let expected_revision = registry_revision(expect)?;
                let expected_project_revision = match expect_project {
                    Some(value) => value.clone(),
                    None => {
                        let registry = crate::registry::load(runtime)?;
                        let project =
                            crate::storage::selected(&api, &registry, &selector.context())?;
                        crate::registry::load_project(&registry, &project.id)?.revision
                    }
                };
                crate::SpecRequest::BranchInit(crate::InitBranchRequest {
                    selector: selector.context(),
                    name: name.clone(),
                    from: from.clone(),
                    expected_revision,
                    expected_project_revision,
                })
            }
            _ => {
                return Err(crate::SpecError::new(
                    "invalid_request",
                    "unknown management operation",
                ));
            }
        };
        api.execute(request)
    })())
}

/// CLI adapter boundary: parse before reading cwd, as the standalone CLI always did.
/// Embedded domain callers should use [`run_cli`] with their own runtime instead.
pub fn run_cli_from_process(
    argv: &[OsString],
    input: &mut dyn std::io::Read,
) -> crate::CommandOutput {
    let cli = match parse(argv) {
        Ok(cli) => cli,
        Err(output) => return output,
    };
    let runtime = match crate::Runtime::from_process() {
        Ok(runtime) => runtime,
        Err(e) => {
            let hook = matches!(cli.command, Command::Hook { .. });
            let suffix = if hook { " — hook skipped" } else { "" };
            return crate::CommandOutput {
                exit: if hook { exit::OK } else { exit::USAGE },
                stdout: Vec::new(),
                stderr: format!("vibe-spec: cannot read the current directory: {e}{suffix}\n")
                    .into_bytes(),
            };
        }
    };
    execute(cli, argv, &runtime, input)
}

fn parse(argv: &[OsString]) -> Result<Cli, crate::CommandOutput> {
    Cli::try_parse_from(argv).map_err(|e| {
        let code = u8::try_from(e.exit_code()).unwrap_or(exit::USAGE);
        let (stdout, stderr) = if e.use_stderr() {
            (Vec::new(), e.to_string().into_bytes())
        } else {
            (e.to_string().into_bytes(), Vec::new())
        };
        crate::CommandOutput {
            exit: code,
            stdout,
            stderr,
        }
    })
}

fn execute(
    cli: Cli,
    argv: &[OsString],
    runtime: &crate::Runtime,
    input: &mut dyn std::io::Read,
) -> crate::CommandOutput {
    if matches!(cli.command, Command::Mcp { .. }) {
        return crate::CommandOutput {
            exit: exit::USAGE,
            stdout: Vec::new(),
            stderr: b"vibe-spec: MCP requires the streaming process adapter\n".to_vec(),
        };
    }
    let cx = crate::api::Execution::new(runtime);
    let cwd = &runtime.cwd;
    if cx.var_os("SPEC_DIR").is_some() {
        diag!(
            &cx,
            "vibe-spec: SPEC_DIR is ignored; set VIBE_SPEC_ROOT instead"
        );
    }
    if matches!(&cli.command, Command::Hook { .. })
        && (cli.project.is_some() || cli.checkout.is_some() || cli.branch.is_some())
    {
        diag!(
            &cx,
            "invalid_request: hooks do not support --project, --checkout or --branch context selectors"
        );
        return cx.finish(exit::FINDINGS, Vec::new());
    }
    if let Command::Hook { event } = &cli.command {
        let mut stdout = Vec::new();
        let code = match event {
            HookEvent::Guard => {
                let mut stdin = Vec::new();
                let _ = std::io::Read::read_to_end(input, &mut stdin);
                hook::guard(&cx, &stdin).0
            }
            HookEvent::Validate => {
                let mut stdin = Vec::new();
                let _ = std::io::Read::read_to_end(input, &mut stdin);
                let (code, out) = hook::validate(&cx, &stdin, cwd);
                stdout = out;
                code
            }
            HookEvent::SessionStart => {
                let mut stdin = Vec::new();
                let _ = std::io::Read::read_to_end(input, &mut stdin);
                let harness = hook::Harness::detect(|k| cx.var_os(k).is_some());
                let (code, out) = hook::session_start(&cx, &stdin, cwd, harness);
                stdout = out;
                code
            }
        };
        return cx.finish(code, stdout);
    }
    let mut domain_runtime = runtime.clone();
    if let Some(root) = &cli.root {
        domain_runtime
            .env
            .insert("VIBE_SPEC_ROOT".into(), root.as_os_str().to_owned());
    }
    if matches!(&cli.command, Command::Reconcile { input: Some(_), .. })
        && (cli.project.is_some()
            || cli.checkout.is_some()
            || cli.branch.is_some()
            || cli.root.is_some())
    {
        diag!(
            &cx,
            "invalid_request: exact plan input cannot be combined with selector overrides"
        );
        return cx.finish(exit::FINDINGS, Vec::new());
    }
    if is_management(&cli.command)
        && (cli.project.is_some() || cli.checkout.is_some() || cli.branch.is_some())
    {
        diag!(
            &cx,
            "invalid_request: management context selectors must follow the command; for example: document write PATH --project ID"
        );
        return cx.finish(exit::FINDINGS, Vec::new());
    }
    if let Some(result) = management(&cli.command, &domain_runtime, input) {
        return match result {
            Ok(response) => cx.finish(exit::OK, format!("{}\n", response.data).into_bytes()),
            Err(error) => {
                diag!(&cx, "{error}");
                cx.finish(exit::FINDINGS, Vec::new())
            }
        };
    }
    let selector = crate::ContextSelector {
        project: cli.project.clone(),
        checkout: cli.checkout.clone(),
        branch: cli.branch.clone(),
    };
    let registered = crate::registry::store_root(&domain_runtime)
        .map(|root| {
            !std::fs::symlink_metadata(root.join("spec/registry.json"))
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        })
        .unwrap_or(true);
    let identity = if registered || selector != crate::ContextSelector::default() {
        match crate::context::registered_identity(&domain_runtime, &selector) {
            Ok(identity) => identity,
            Err(error) => {
                diag!(&cx, "{error}");
                return cx.finish(exit::FINDINGS, Vec::new());
            }
        }
    } else {
        None
    };
    let selected = if registered
        || selector != crate::ContextSelector::default()
        || matches!(
            cli.command,
            Command::Validate {
                spec_only: true,
                ..
            } | Command::CheckDrift {
                spec_only: true,
                ..
            }
        ) {
        crate::context::resolve_for_routing(&domain_runtime, &selector).map(Some)
    } else {
        Ok(None)
    };

    let mut selected_origin = None;
    match selected {
        Ok(Some(context)) if context.mode == crate::StorageMode::GlobalFirst => {
            let result = managed_legacy(
                &cx,
                &domain_runtime,
                &selector,
                &context,
                &cli.command,
                argv,
            );
            return match result {
                Ok((code, out)) => cx.finish(code, out),
                Err(error) => {
                    diag!(&cx, "{error}");
                    cx.finish(exit::FINDINGS, Vec::new())
                }
            };
        }
        Ok(Some(context)) if context.project_id.is_some() => {
            let result = repo_legacy(
                &cx,
                &domain_runtime,
                &selector,
                &context,
                &cli.command,
                argv,
            );
            if let Some(result) = result {
                return match result {
                    Ok((code, out)) => cx.finish(code, out),
                    Err(error) => {
                        diag!(&cx, "{error}");
                        cx.finish(exit::FINDINGS, Vec::new())
                    }
                };
            }
            if selector != crate::ContextSelector::default() {
                selected_origin = context.checkout;
            }
        }
        Ok(Some(context))
            if matches!(
                cli.command,
                Command::Validate {
                    spec_only: true,
                    ..
                } | Command::CheckDrift {
                    spec_only: true,
                    ..
                }
            ) =>
        {
            return match managed_legacy(
                &cx,
                &domain_runtime,
                &selector,
                &context,
                &cli.command,
                argv,
            ) {
                Ok((code, out)) => cx.finish(code, out),
                Err(error) => {
                    diag!(&cx, "{error}");
                    cx.finish(exit::FINDINGS, Vec::new())
                }
            };
        }
        Ok(Some(context)) if selector != crate::ContextSelector::default() => {
            selected_origin = context.checkout;
        }
        Err(error)
            if identity.is_some()
                || selector != crate::ContextSelector::default()
                || matches!(
                    error.code.as_str(),
                    "context_mismatch"
                        | "invalid_registry"
                        | "ambiguous_project"
                        | "invalid_project"
                ) =>
        {
            diag!(&cx, "{error}");
            return cx.finish(exit::FINDINGS, Vec::new());
        }
        _ => {}
    }
    let env_root = cx.var_os("VIBE_SPEC_ROOT");
    let found = root::discover(
        selected_origin.as_deref().unwrap_or(cwd),
        cli.root.as_deref(),
        env_root.as_deref(),
    );
    if let Ok(root) = &found
        && let Some(warning) = root::both_roots_warning(root)
    {
        diag!(&cx, "{warning}");
    }
    let legacy_api = match crate::SpecApi::new(domain_runtime.clone()) {
        Ok(api) => api,
        Err(error) => {
            diag!(&cx, "{error}");
            return cx.finish(exit::FINDINGS, Vec::new());
        }
    };
    let (code, out) = match cli.command {
        Command::Root { json } => root::command(&cx, &found, json),
        Command::ListSpecs { args: _ } => {
            list_specs::run(&cx, &found, &raw_tail(argv.iter().cloned()))
        }
        Command::LessonsFor { args: _ } => {
            lessons::run(&cx, &found, &raw_tail(argv.iter().cloned()))
        }
        Command::ScanMerges { args: _ } => {
            scan_merges::run(&cx, &found, cwd, &raw_tail(argv.iter().cloned()))
        }
        Command::Validate {
            spec_only: _,
            strict,
            format,
            json,
            changed,
            file,
            event,
        } => validate::run(
            &cx,
            &found,
            cwd,
            &validate::Options {
                strict,
                format: if json { validate::Format::Json } else { format },
                changed,
                file,
                event,
            },
        ),
        Command::Plan { feature, json } => query::plan(&cx, &found, &feature, json),
        Command::Feature { name, json } => query::feature(&cx, &found, &name, json),
        Command::CheckDrift {
            pre_merge,
            json,
            spec_only: _,
        } => drift::run(&cx, &found, cwd, &drift::Options { pre_merge, json }),
        Command::Promote { args: _ } => legacy_result(
            &cx,
            promote::repo_cli(
                &legacy_api,
                &selector,
                &cx,
                &found,
                cwd,
                &raw_tail(argv.iter().cloned()),
            ),
        ),
        Command::Export {
            openspec: dir,
            json,
        } => openspec::export(&cx, &found, cwd, &dir, json),
        Command::Import {
            openspec: dir,
            feature,
            json,
        } => legacy_result(
            &cx,
            openspec::repo_import_cli(
                &legacy_api,
                &selector,
                &cx,
                &found,
                cwd,
                &dir,
                feature.as_deref(),
                json,
            ),
        ),
        Command::Trace { since, json } => trace::run(&cx, &found, since.as_deref(), json),
        Command::Init { to, json } => legacy_result(
            &cx,
            init::repo_cli(
                &legacy_api,
                &selector,
                &cx,
                &found,
                cwd,
                to.as_deref(),
                json,
            ),
        ),
        Command::Setup {
            git_hooks,
            opencode,
            agents_md,
            json,
        } => setup::run(
            &cx,
            &found,
            cwd,
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
            &cx,
            &found,
            cwd,
            &migrate::Options {
                to,
                layered: layout.is_some(),
                dry_run,
                check,
                json,
            },
        ),
        Command::Mcp { .. }
        | Command::Hook { .. }
        | Command::Project { .. }
        | Command::Branch { .. }
        | Command::Context { .. } => (exit::OK, Vec::new()),
        Command::Reconcile { .. } | Command::Document { .. } | Command::Snapshot { .. } => {
            (exit::OK, Vec::new())
        }
    };
    cx.finish(code, out)
}

/// The argv after the subcommand token, exactly as typed. clap strips a
/// leading `--` from trailing arguments, but the legacy commands must see it
/// (the oracles reject it as `Unknown option`). Every legacy subcommand
/// reads its arguments through this; `validate` no longer does (clap parses
/// its flags). Non-UTF-8 arguments are read lossily,
/// never a panic.
fn raw_tail(argv: impl IntoIterator<Item = OsString>) -> Vec<String> {
    let args: Vec<_> = argv.into_iter().skip(1).collect();
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        let arg = arg.to_string_lossy();
        if ["--root", "--project", "--checkout", "--branch"].contains(&arg.as_ref()) {
            at += 2;
        } else if ["--root=", "--project=", "--checkout=", "--branch="]
            .iter()
            .any(|p| arg.starts_with(p))
        {
            at += 1;
        } else {
            at += 1;
            break;
        }
    }
    args.into_iter()
        .skip(at)
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
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

pub(crate) fn refresh_notice(cx: &crate::api::Execution<'_>, context: &crate::SpecContext) {
    let report = &context.refresh;
    if !report.applied.is_empty()
        || !report.conflict.is_empty()
        || !report.needs_input.is_empty()
        || !report.diagnostics.is_empty()
    {
        diag!(
            cx,
            "refresh: {}",
            serde_json::json!({"context":crate::storage::context_value(context),"report":report.value()})
        );
    }
}
pub(crate) fn read_contextual<T>(
    api: &crate::SpecApi,
    selector: &crate::ContextSelector,
    cx: &crate::api::Execution<'_>,
    f: impl FnOnce(&crate::SpecContext) -> Result<T, crate::SpecError>,
) -> Result<(crate::SpecContext, T), crate::SpecError> {
    let result = crate::transaction::read_typed(api, selector, f)?;
    refresh_notice(cx, &result.0);
    Ok(result)
}

fn managed_legacy(
    cx: &crate::api::Execution<'_>,
    runtime: &crate::Runtime,
    selector: &crate::ContextSelector,
    context: &crate::SpecContext,
    command: &Command,
    argv: &[OsString],
) -> Result<(u8, Vec<u8>), crate::SpecError> {
    use crate::{SpecError, SpecRequest as R};
    let api = crate::SpecApi::new(runtime.clone())?;
    let scope = |spec_only| {
        if spec_only {
            crate::ValidationScope::SpecOnly
        } else {
            crate::ValidationScope::WithCode
        }
    };
    let answer = |response: crate::SpecResponse, json: bool| -> (u8, Vec<u8>) {
        if let Some(context) = &response.context {
            refresh_notice(cx, context);
        }
        let code = if response.data["errors"].as_u64().unwrap_or(0) > 0 {
            exit::FINDINGS
        } else {
            exit::OK
        };
        if json {
            return (code, format!("{}\n", response.data).into_bytes());
        }
        (code, format!("{}\n", response.data).into_bytes())
    };
    match command {
        Command::Root { json } => {
            let (_, answer) = read_contextual(&api, selector, cx, |context| {
                let data = serde_json::json!({"v":1,"root":context.spec_root,"layout":crate::layered::detect(&context.spec_root).as_str(),"source":"config","depth":0});
                Ok((
                    exit::OK,
                    if *json {
                        format!("{data}\n").into_bytes()
                    } else {
                        format!("{}\n", context.spec_root.display()).into_bytes()
                    },
                ))
            })?;
            Ok(answer)
        }
        Command::Validate {
            strict,
            format,
            json,
            changed,
            file,
            event,
            spec_only,
        } => {
            let result = read_contextual(&api, selector, cx, |context| {
                crate::validate::legacy_context_result(
                    runtime,
                    context,
                    scope(*spec_only),
                    *strict,
                    *changed,
                    file.as_deref(),
                )
            })?
            .1;
            for diagnostic in &result.diagnostics {
                diag!(cx, "{}", diagnostic.message);
            }
            let format = if *json {
                validate::Format::Json
            } else {
                *format
            };
            Ok(crate::validate::render_managed(
                result,
                format,
                event.as_deref(),
            ))
        }
        Command::Plan { feature, json } => {
            let response = api.execute(R::Plan(crate::PlanRequest {
                selector: selector.clone(),
                feature: feature.clone(),
            }))?;
            if *json {
                Ok(answer(response, true))
            } else {
                if let Some(context) = &response.context {
                    refresh_notice(cx, context);
                }
                let text = response.data["rows"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|r| {
                        format!(
                            "{}\t{}{}\n",
                            r["id"].as_str().unwrap_or_default(),
                            r["status"].as_str().unwrap_or_default(),
                            r["title"]
                                .as_str()
                                .map_or(String::new(), |t| format!("\t{t}"))
                        )
                    })
                    .collect::<String>();
                Ok((0, text.into_bytes()))
            }
        }
        Command::Feature { name, json } => {
            let response = api.execute(R::Feature(crate::FeatureRequest {
                selector: selector.clone(),
                feature: name.clone(),
            }))?;
            if *json {
                Ok(answer(response, true))
            } else {
                if let Some(context) = &response.context {
                    refresh_notice(cx, context);
                }
                let mut text = format!(
                    "layer: {}\n",
                    response.data["layer"].as_str().unwrap_or_default()
                );
                for d in response.data["docs"].as_array().into_iter().flatten() {
                    text += &format!(
                        "{}\t{}\n",
                        d["kind"].as_str().unwrap_or_default(),
                        d["path"].as_str().unwrap_or_default()
                    );
                }
                Ok((0, text.into_bytes()))
            }
        }
        Command::CheckDrift {
            pre_merge,
            json,
            spec_only,
        } => crate::drift::managed_cli(&api, selector, scope(*spec_only), *pre_merge, *json, cx),
        Command::Trace { since, json } => {
            crate::trace::managed_cli(&api, selector, since.as_deref(), *json, cx)
        }
        Command::ListSpecs { .. } => {
            let raw = raw_tail(argv.iter().cloned());
            if wants_help(&raw) {
                return Ok(list_specs::run(cx, &Err(root::Miss::NotFound), &raw));
            }
            let (_, text) = read_contextual(&api, selector, cx, |context| {
                Ok(crate::list_specs::render_files(&crate::documents::collect(
                    context,
                )?))
            })?;
            Ok((0, text))
        }
        Command::LessonsFor { .. } => {
            let args = raw_tail(argv.iter().cloned());
            let (_, result) = read_contextual(&api, selector, cx, |context| {
                crate::lessons::render_managed(
                    &crate::documents::bytes(context, "spec/lessons.md")?.unwrap_or_default(),
                    &args,
                )
            })?;
            Ok(result)
        }
        Command::ScanMerges { .. } => {
            let args = raw_tail(argv.iter().cloned());
            let (_, result) = read_contextual(&api, selector, cx, |context| {
                crate::scan_merges::render_managed(&crate::documents::collect(context)?, &args)
            })?;
            Ok(result)
        }
        Command::Promote { .. } => {
            crate::promote::managed_cli(&api, selector, &raw_tail(argv.iter().cloned()))
        }
        Command::Init { to, json } => {
            if to
                .as_ref()
                .is_some_and(|p| runtime.cwd.join(p) != context.spec_root)
            {
                return Err(SpecError::new(
                    "context_mismatch",
                    "global init targets the selected spec root",
                ));
            }
            Ok(answer(
                api.execute(R::Init(crate::InitRequest {
                    selector: selector.clone(),
                    expected_revision: api.resolve_context(selector)?.revision_token,
                }))?,
                *json,
            ))
        }
        Command::Export { openspec, json } => {
            crate::openspec::managed_export_cli(&api, selector, openspec, *json, cx)
        }
        Command::Import {
            openspec,
            feature,
            json,
        } => crate::openspec::managed_import_cli(
            &api,
            selector,
            openspec,
            feature.as_deref(),
            *json,
            cx,
        ),
        _ => {
            let _ = cx;
            Err(SpecError::new(
                "unsupported_mode",
                "this CLI-only operation is not available for global spec storage",
            ))
        }
    }
}

fn repo_legacy(
    cx: &crate::api::Execution<'_>,
    runtime: &crate::Runtime,
    selector: &crate::ContextSelector,
    context: &crate::SpecContext,
    command: &Command,
    argv: &[OsString],
) -> Option<Result<(u8, Vec<u8>), crate::SpecError>> {
    let api = match crate::SpecApi::new(runtime.clone()) {
        Ok(api) => api,
        Err(error) => return Some(Err(error)),
    };
    let checkout = context.checkout.as_deref()?;
    let override_root = crate::api::Execution::new(runtime)
        .var_os("VIBE_SPEC_ROOT")
        .map(|p| runtime.cwd.join(p));
    let origin = if *selector == crate::ContextSelector::default() {
        runtime.cwd.as_path()
    } else {
        checkout
    };
    let found = root::discover(
        origin,
        None,
        override_root.as_deref().map(std::path::Path::as_os_str),
    );
    let cwd = &runtime.cwd;
    let raw = raw_tail(argv.iter().cloned());
    let read = |f: &dyn Fn() -> (u8, Vec<u8>)| {
        read_contextual(&api, selector, cx, |_| Ok(f())).map(|(_, out)| out)
    };
    Some(match command {
        Command::Root { .. } => return None,
        Command::Plan { feature, json } => read(&|| query::plan(cx, &found, feature, *json)),
        Command::Feature { name, json } => read(&|| query::feature(cx, &found, name, *json)),
        Command::LessonsFor { .. } => read(&|| lessons::run(cx, &found, &raw)),
        Command::ListSpecs { .. } => read(&|| list_specs::run(cx, &found, &raw)),
        Command::ScanMerges { .. } => read(&|| scan_merges::run(cx, &found, cwd, &raw)),
        Command::Trace { since, json } => {
            read(&|| trace::run_registered(cx, &found, since.as_deref(), *json))
        }
        Command::CheckDrift {
            pre_merge,
            json,
            spec_only: false,
        } => read(&|| {
            drift::run_registered(
                cx,
                &found,
                cwd,
                &drift::Options {
                    pre_merge: *pre_merge,
                    json: *json,
                },
            )
        }),
        Command::Validate {
            strict,
            format,
            json,
            changed,
            file,
            event,
            spec_only: false,
        } => read(&|| {
            validate::run_registered(
                cx,
                &found,
                cwd,
                &validate::Options {
                    strict: *strict,
                    format: if *json {
                        validate::Format::Json
                    } else {
                        *format
                    },
                    changed: *changed,
                    file: file.clone(),
                    event: event.clone(),
                },
            )
        }),
        Command::Validate { .. } | Command::CheckDrift { .. } => {
            managed_legacy(cx, runtime, selector, context, command, argv)
        }
        Command::Init { to, json } => {
            init::repo_cli(&api, selector, cx, &found, cwd, to.as_deref(), *json)
        }
        Command::Import {
            openspec,
            feature,
            json,
        } => openspec::repo_import_cli(
            &api,
            selector,
            cx,
            &found,
            cwd,
            openspec,
            feature.as_deref(),
            *json,
        ),
        Command::Export { openspec, json } => {
            read(&|| openspec::export(cx, &found, cwd, openspec, *json))
        }
        Command::Promote { .. } => promote::repo_cli(&api, selector, cx, &found, cwd, &raw),
        _ => return None,
    })
}

fn legacy_result(
    cx: &crate::api::Execution<'_>,
    result: Result<(u8, Vec<u8>), crate::SpecError>,
) -> (u8, Vec<u8>) {
    match result {
        Ok(answer) => answer,
        Err(error) => {
            diag!(cx, "{error}");
            (exit::FINDINGS, Vec::new())
        }
    }
}

/// Streaming binary boundary shared by the central and compatibility executables.
/// Parsing precedes cwd capture; buffered library APIs never claim process stdio.
pub fn run_process(argv: &[OsString]) -> u8 {
    use std::io::Write;
    let out = match parse(argv) {
        Err(out) => out,
        Ok(cli) if matches!(cli.command, Command::Mcp { .. }) => {
            let result = crate::Runtime::from_process()
                .map_err(|e| crate::SpecError::new("invalid_context", e.to_string()))
                .and_then(|mut runtime| {
                    if let Some(root) = cli.root {
                        runtime.env.insert(
                            "VIBE_SPEC_ROOT".into(),
                            runtime.cwd.join(root).into_os_string(),
                        );
                    }
                    crate::mcp::serve_selected(
                        runtime,
                        crate::ContextSelector {
                            project: cli.project,
                            checkout: cli.checkout,
                            branch: cli.branch,
                        },
                    )
                });
            return match result {
                Ok(()) => exit::OK,
                Err(e) => {
                    let _ = writeln!(std::io::stderr().lock(), "vibe-spec: {e}");
                    exit::FINDINGS
                }
            };
        }
        Ok(_) => run_cli_from_process(argv, &mut std::io::stdin()),
    };
    let _ = std::io::stdout().lock().write_all(&out.stdout);
    let _ = std::io::stderr().lock().write_all(&out.stderr);
    out.exit
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
