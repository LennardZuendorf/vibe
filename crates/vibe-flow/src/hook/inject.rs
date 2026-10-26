//! `user-prompt-submit` is composed by `vibe-instruct` (D25); flow keeps the
//! guard and Stop. This is an explicit no-op until the F12 shim flip, so the
//! event name is wired without claiming ownership of the stream.

use super::Outcome;

/// The composition event's placeholder: exit 0 with no streams, never read
/// stdin (the caller must not read it for this event).
pub fn run() -> Outcome {
    Outcome::new(0, String::new(), String::new())
}
