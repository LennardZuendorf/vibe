//! Process exit codes shared by every vibe binary (contracts/exit-codes.json).

/// Success.
pub const OK: u8 = 0;
/// Findings or a refused action.
pub const FINDINGS: u8 = 1;
/// Usage error.
pub const USAGE: u8 = 2;
/// A query found no tree to answer from.
pub const NO_ROOT: u8 = 3;
