//! Command surfaces. Each returns an [`Outcome`] so the entry point only
//! writes bytes and maps the exit code.

pub mod policy;
pub mod state;

/// One command's result: an exit code and the bytes for each stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The process exit code.
    pub code: u8,
    /// Bytes for stdout.
    pub stdout: String,
    /// Bytes for stderr.
    pub stderr: String,
}

impl Outcome {
    /// A result with the given code and streams.
    pub fn new(code: u8, stdout: String, stderr: String) -> Outcome {
        Outcome {
            code,
            stdout,
            stderr,
        }
    }
}
