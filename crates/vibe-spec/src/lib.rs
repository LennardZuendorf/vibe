//! `vibe-spec`: the spec tool. F16 ports the read-only spec scripts and adds
//! root discovery and the Spec JSON v1 queries; F17 adds the check registry,
//! strictness, the F17 validators and `trace`.
/// Concatenates byte-slice-like values (`&str`, `&[u8]`, `Vec<u8>`, `String`) into one `Vec<u8>`.
macro_rules! cat {
    ($($part:expr),* $(,)?) => {{
        let mut v: Vec<u8> = Vec::new();
        $(v.extend_from_slice(AsRef::<[u8]>::as_ref(&$part));)*
        v
    }};
}

macro_rules! diag {
    ($cx:expr, $($part:tt)*) => { $cx.diagnostic(format_args!($($part)*)) };
}

mod adherence;
mod branch;
mod check;
mod content;
mod control;
pub use control::InvocationControl;
mod context;
mod delta;
mod documents;
mod drift;
mod execution;
mod frontmatter;
mod git;
mod hook;
mod init;
mod layered;
mod lessons;
mod list_specs;
mod literal;
pub mod mcp;
mod migrate;
mod openspec;
mod pattern;
mod promote;
mod query;
pub mod reconcile;
pub mod refresh;
mod registry;
mod report;
mod root;
mod scan_merges;
mod setup;
mod storage;
mod superpowers;
mod text;
mod trace;
mod transaction;
mod transcript;
mod validate;

mod api;
mod cli;
pub use api::{
    AttachProjectRequest, CommandOutput, InitBranchRequest, InitProjectRequest,
    RemoveDocumentRequest, Runtime, SnapshotRequest, SpecApi, SpecError, SpecRequest, SpecResponse,
    WriteDocumentRequest,
};
use cli::wants_help;
pub use cli::{run_cli, run_cli_from_process, run_process};
pub use context::{
    BranchId, ContextSelector, ProjectId, RecordId, SpecContext, StorageMode, resolve,
};
pub use documents::{DocumentContent, DocumentList, DocumentMetadata, DocumentRead};

pub use validate::{
    Diagnostic, ValidateRequest, ValidationFinding, ValidationResult, ValidationScope,
};

pub use lessons::LessonsRequest;
pub use query::{FeatureRequest, PlanRequest};

pub use drift::DriftRequest;
pub use trace::TraceRequest;

pub use init::InitRequest;
pub use openspec::{BundleFile, ExportRequest, ImportRequest, OpenSpecBundle};

pub use promote::PromoteRequest;

#[doc(hidden)]
pub use hook::parse as parse_hook_payload;

pub use reconcile::{
    ConflictContent, ReconcilePlan, ReconcileRecord, ReconcileRequest, ReconcileState,
    ResolutionFile, ResolveRequest,
};
