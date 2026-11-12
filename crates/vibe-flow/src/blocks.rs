//! The marker-block grammar the flow half reads (`blocks.mjs`'s
//! `extractBlock`, byte-for-byte).
//!
//! Orders live in the linked skill's `SKILL.md` as
//! `<!-- vibe:orders:<state> -->` … `<!-- /vibe:orders -->`, and the doctrine as
//! `<!-- vibe:doctrine -->` … `<!-- /vibe:doctrine -->`. A block whose opener
//! has no closer before EOF is *absent*, not "the rest of the file": the bash
//! oracle's `sed -n '/open/,/close/p'` leaks the tail in that case, and the
//! engine (and this port) deliberately does not.

/// The block body for `id`, marker lines excluded, or `None` when the pair is
/// absent (no opener, or no closer after it).
pub fn extract(text: &str, id: &str) -> Option<String> {
    let opener = format!("<!-- {id} -->");
    let closers = closer_candidates(id);
    let lines: Vec<&str> = text.split('\n').collect();
    let open = lines.iter().position(|line| *line == opener)?;
    let mut close = None;
    for (index, line) in lines.iter().enumerate().skip(open + 1) {
        if closers.iter().any(|closer| *line == closer.as_str()) {
            close = Some(index);
            break;
        }
    }
    let close = close?;
    Some(lines[open + 1..close].join("\n"))
}

/// A block id with more than two `:`-segments may close on the legacy
/// asymmetric form (`vibe:orders:feature.impl` → `/vibe:orders`), which is what
/// the oracle's sed range and every shipped SKILL.md actually use.
fn closer_candidates(id: &str) -> Vec<String> {
    let exact = format!("<!-- /{id} -->");
    let parts: Vec<&str> = id.split(':').collect();
    if parts.len() > 2 {
        vec![exact, format!("<!-- /{} -->", parts[..2].join(":"))]
    } else {
        vec![exact]
    }
}

/// Mirrors bash `$(...)`: a captured value loses *all* trailing newlines
/// before the caller adds exactly one back. The block grammar itself must not
/// trim (other consumers want the raw body), so the two capture-shaped call
/// sites apply this.
pub fn strip_trailing_newlines(text: &str) -> &str {
    text.trim_end_matches('\n')
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{extract, strip_trailing_newlines};

    #[test]
    fn extracts_the_body_between_the_markers() {
        let text = "a\n<!-- vibe:orders:idle -->\nline 1\nline 2\n<!-- /vibe:orders -->\nz\n";
        assert_eq!(
            extract(text, "vibe:orders:idle").as_deref(),
            Some("line 1\nline 2")
        );
    }

    #[test]
    fn accepts_the_legacy_truncated_closer() {
        let text = "<!-- vibe:orders:feature.impl -->\nbody\n<!-- /vibe:orders -->\n";
        assert_eq!(
            extract(text, "vibe:orders:feature.impl").as_deref(),
            Some("body")
        );
    }

    #[test]
    fn a_missing_closer_is_absent_not_the_file_tail() {
        let text = "<!-- vibe:orders:idle -->\nleaked?\n";
        assert_eq!(extract(text, "vibe:orders:idle"), None);
    }

    #[test]
    fn an_absent_opener_is_none() {
        assert_eq!(extract("<!-- vibe:doctrine -->\nx\n", "vibe:orders:idle"), None);
    }

    #[test]
    fn markers_must_be_whole_lines() {
        let text = "x <!-- vibe:orders:idle -->\nbody\n<!-- /vibe:orders -->\n";
        assert_eq!(extract(text, "vibe:orders:idle"), None);
    }

    #[test]
    fn captures_strip_every_trailing_newline() {
        assert_eq!(strip_trailing_newlines("a\n\n\n"), "a");
        assert_eq!(strip_trailing_newlines("a"), "a");
        assert_eq!(strip_trailing_newlines(""), "");
    }
}
