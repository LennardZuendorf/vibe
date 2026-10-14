//! Differential parity harness (F3) and workspace structure checks (F4).
//!
//! A suite names commands, an oracle argv and a candidate argv for each, and
//! the corpus classes they run over. Every run stages a case into a sandbox,
//! runs the oracle, restages, runs the candidate, and compares exit code,
//! stdout bytes and the bytes of every file left behind.
pub mod harness;
pub mod sandbox;
pub mod suite;
pub mod workspace;
