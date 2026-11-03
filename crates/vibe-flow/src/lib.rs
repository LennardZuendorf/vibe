//! `vibe-flow`: the flow half's core — the cursor, the write policy and the
//! state machine, each mirroring its frozen JS/bash oracle.
//!
//! Layout resolution lives in one place ([`paths::FlowPaths`]); every other
//! module is pure or takes its paths as an argument.

pub mod cli;
pub mod cmd;
pub mod cursor;
pub mod hook;
pub mod machine;
pub mod paths;
pub mod policy;
