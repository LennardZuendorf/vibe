//! Shared internals for the vibe tools.
//!
//! Holds no tool vocabulary: no flow state names, no spec paths, no
//! instruct channel names. Tools pass those in.
pub mod error;
pub mod exit;

pub use error::{Error, Result};
