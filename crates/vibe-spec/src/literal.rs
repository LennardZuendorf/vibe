//! Literal detection for `vibe-spec migrate` (P4-R12, P4-R16a).
//!
//! A literal is the legacy root name standing alone: not preceded or followed
//! by a letter, digit, `_` or `-` (the P4-R12 regex, applied per line and
//! matched here by hand, since there is no regex crate). Deliberately tighter
//! than that regex: a `.` before the name (`..` + name), or a `.` and a word
//! character after it (a file extension such as `.ts`), is not a literal.
//! Two exemptions:
//!
//! - a line tag: the line, minus trailing whitespace, ends with
//!   `<!-- literal -->`, `// literal` or `# literal`. The two short tags must
//!   start the line or follow whitespace, so a heading such as `## literal`
//!   is not a tag;
//! - a file marker: `vibe-spec: literal-file` as the content of one of the
//!   first five lines, optionally inside a comment (see [`file_exempt`]).
use crate::text::{is_space, trim_end_space, trim_space};

/// The legacy root name; the only needle `migrate` rewrites.
pub const LEGACY: &str = ".spec"; // literal

/// The file-level marker text.
const FILE_MARKER: &[u8] = b"vibe-spec: literal-file";

fn word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

/// The line carries a literal tag.
pub fn tagged(line: &[u8]) -> bool {
    let t = trim_end_space(line);
    if t.ends_with(b"<!-- literal -->") {
        return true;
    }
    [b"// literal".as_slice(), b"# literal".as_slice()]
        .iter()
        .any(|tag| {
            t.strip_suffix(*tag)
                .is_some_and(|rest| rest.last().is_none_or(|c| is_space(*c)))
        })
}

/// Byte offsets of `needle` at word boundaries; empty when the line is tagged.
fn find_needle(line: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() || tagged(line) {
        return Vec::new();
    }
    line.windows(needle.len())
        .enumerate()
        .filter(|(at, w)| {
            let after = at + needle.len();
            *w == needle
                && (*at == 0 || line.get(*at - 1).is_none_or(|b| !word(*b) && *b != b'.'))
                && line.get(after).is_none_or(|b| !word(*b))
                && !(line.get(after) == Some(&b'.')
                    && line.get(after + 1).is_some_and(|b| word(*b)))
        })
        .map(|(at, _)| at)
        .collect()
}

/// Byte offsets of each literal in `line`; empty when the line is tagged.
pub fn find(line: &[u8]) -> Vec<usize> {
    find_needle(line, LEGACY.as_bytes())
}

/// `line` with each `from` literal replaced by `to`. A literal followed by
/// `/<old>` for a rename `(old, new)` becomes `to/<new>`, when `<old>` ends at a
/// word boundary. A tagged line is returned unchanged.
pub fn rewrite(line: &[u8], from: &str, to: &str, renames: &[(String, String)]) -> Vec<u8> {
    let hits = find_needle(line, from.as_bytes());
    let mut out = Vec::with_capacity(line.len());
    let mut at = 0;
    for hit in hits {
        let Some(between) = line.get(at..hit) else {
            continue;
        };
        out.extend_from_slice(between);
        out.extend_from_slice(to.as_bytes());
        at = hit + from.len();
        let rest = line.get(at..).unwrap_or_default();
        for (old, new) in renames {
            let want = format!("/{old}");
            if rest.starts_with(want.as_bytes()) && rest.get(want.len()).is_none_or(|b| !word(*b)) {
                out.extend_from_slice(format!("/{new}").as_bytes());
                at += want.len();
                break;
            }
        }
    }
    out.extend_from_slice(line.get(at..).unwrap_or_default());
    out
}

/// A line that is exactly the file marker, alone or inside one comment:
/// `<!-- … -->`, `/* … */`, or after a `//`, `#`, `--` or `;` opener.
fn is_marker_line(line: &[u8]) -> bool {
    let mut t = trim_space(line);
    for (open, close) in [
        (b"<!--".as_slice(), b"-->".as_slice()),
        (b"/*".as_slice(), b"*/".as_slice()),
        (b"//".as_slice(), b"".as_slice()),
        (b"#".as_slice(), b"".as_slice()),
        (b"--".as_slice(), b"".as_slice()),
        (b";".as_slice(), b"".as_slice()),
    ] {
        if let Some(inner) = t.strip_prefix(open) {
            t = trim_space(inner.strip_suffix(close).unwrap_or(inner));
            break;
        }
    }
    t == FILE_MARKER
}

/// The file holds the file-level marker in its first five lines.
pub fn file_exempt(text: &[u8]) -> bool {
    text.split(|b| *b == b'\n').take(5).any(is_marker_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> Vec<usize> {
        find(s.replace('@', LEGACY).as_bytes())
    }

    #[test]
    fn offsets_point_at_the_dot() {
        assert_eq!(at("@"), [0]);
        assert_eq!(at("x ./@ y @/z"), [4, 12]);
        assert_eq!(at("\"@\""), [1]);
    }

    #[test]
    fn boundaries_and_tags() {
        assert!(at("strategy.spec x.spec.ts tech-spec.md .vibe/spec.json").is_empty());
        assert!(at("@-x _@ a@").is_empty());
        assert!(at("@/x <!-- literal -->").is_empty());
        assert!(at("@ // literal  ").is_empty());
        assert!(at("@ # literal").is_empty());
        assert_eq!(at("@ ## literal"), [0]);
        assert_eq!(at("@ x// literal"), [0]);
        assert!(at("..@ @.ts @.js @.md").is_empty());
        assert_eq!(at("ends @."), [5]);
    }

    #[test]
    fn rewrite_swaps_each_literal_and_applies_renames() {
        let line = format!("see {LEGACY}/tech-cli.md and {LEGACY}/tech-cli.md.bak and {LEGACY}/");
        let renames = [("tech-cli.md".to_string(), "tech/cli.md".to_string())];
        let got = rewrite(line.as_bytes(), LEGACY, "docs/spec", &renames);
        assert_eq!(
            got,
            b"see docs/spec/tech/cli.md and docs/spec/tech/cli.md.bak and docs/spec/"
        );
        let tagged = format!("{LEGACY} // literal");
        assert_eq!(
            rewrite(tagged.as_bytes(), LEGACY, "docs/spec", &[]),
            tagged.as_bytes()
        );
    }

    #[test]
    fn file_marker_is_anchored_to_the_first_five_lines() {
        assert!(file_exempt(b"# t\n<!-- vibe-spec: literal-file -->\n"));
        assert!(file_exempt(b"// vibe-spec: literal-file\n"));
        assert!(file_exempt(b"1\n2\n3\n4\nvibe-spec: literal-file\n"));
        assert!(!file_exempt(b"1\n2\n3\n4\n5\nvibe-spec: literal-file\n"));
        assert!(!file_exempt(b"see vibe-spec: literal-file here\n"));
        assert!(!file_exempt(b""));
    }
}
