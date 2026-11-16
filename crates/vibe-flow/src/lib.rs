//! `vibe-flow`: the flow half's core — the cursor, the write policy and the
//! state machine, each mirroring its frozen JS/bash oracle.
//!
//! Layout resolution lives in one place ([`paths::FlowPaths`]); every other
//! module is pure or takes its paths as an argument.

pub mod agents_md;
pub mod blocks;
pub mod candidate;
pub mod cli;
pub mod cmd;
pub mod cursor;
pub mod doctor;
pub mod doctrine;
pub mod emit;
pub mod evidence;
pub mod hook;
pub mod init;
pub mod ledger;
pub mod machine;
pub mod orders;
pub mod paths;
pub mod payload;
pub mod policy;
pub mod runtime;
pub mod transition;
