//! Requirement grammar and the delta engine (F18, D34, P3c-R7–R10):
//! OpenSpec's `### Requirement:` blocks and `## ADDED|MODIFIED|REMOVED
//! Requirements` sections inside vibe's own documents. Headers match the way
//! OpenSpec matches them; a requirement's key is its full normalized name.
use std::path::{Path, PathBuf};

use crate::layered::lexical;
use crate::text::{Fences, lossy};

/// What a requirement-bearing `##` section holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `## Requirements`.
    Plain,
    /// `## ADDED Requirements`.
    Added,
    /// `## MODIFIED Requirements`.
    Modified,
    /// `## REMOVED Requirements`.
    Removed,
    /// `## RENAMED Requirements`, read only to refuse it.
    Renamed,
}

impl Kind {
    /// The OpenSpec section word.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Plain => "REQUIREMENTS",
            Kind::Added => "ADDED",
            Kind::Modified => "MODIFIED",
            Kind::Removed => "REMOVED",
            Kind::Renamed => "RENAMED",
        }
    }
}

/// One requirement block: `text[start..end]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub name: Vec<u8>,
    pub start: usize,
    pub end: usize,
}

/// One requirement-bearing section: the header line starts at `header`; the
/// section ends at `end` (exclusive).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub kind: Kind,
    pub header: usize,
    pub end: usize,
    pub blocks: Vec<Block>,
}

/// One delta entry and whether it changed the target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub kind: Kind,
    pub name: Vec<u8>,
    pub applied: bool,
}

/// `(start, end)` of every line, `\n` included; a last line without `\n` counts.
fn spans(text: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = text
            .get(start..)
            .and_then(|rest| rest.iter().position(|&b| b == b'\n'))
            .map_or(text.len(), |i| start + i + 1);
        out.push((start, end));
        start = end;
    }
    out
}

/// A line without its `\n` and `\r`.
fn bare(line: &[u8]) -> &[u8] {
    let l = line.strip_suffix(b"\n").unwrap_or(line);
    l.strip_suffix(b"\r").unwrap_or(l)
}

fn is_ws(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

fn trim_ws(s: &[u8]) -> &[u8] {
    let start = s.iter().position(|&c| !is_ws(c)).unwrap_or(s.len());
    let end = s.iter().rposition(|&c| !is_ws(c)).map_or(start, |i| i + 1);
    s.get(start..end).unwrap_or_default()
}

/// OpenSpec's `normalizeRequirementName`: drop a closing `#` run that
/// follows a space or tab, then trim.
fn normalize(name: &[u8]) -> &[u8] {
    let t = trim_ws(name);
    let hashes = t.iter().rev().take_while(|&&c| c == b'#').count();
    let before = t.get(..t.len() - hashes).unwrap_or_default();
    if hashes > 0 && before.last().is_some_and(|&c| is_ws(c)) {
        trim_ws(before)
    } else {
        t
    }
}

/// The key of a `### Requirement: <name>` line (OpenSpec's
/// `^###\s*Requirement:\s*(.+)\s*$`, case-insensitive keyword); None for
/// any other line or an empty name.
pub fn requirement_name(line: &[u8]) -> Option<Vec<u8>> {
    let rest = bare(line).strip_prefix(b"###")?;
    if rest.first() == Some(&b'#') {
        return None;
    }
    let rest = rest.get(rest.iter().position(|&c| !is_ws(c))?..)?;
    if !rest.get(..12)?.eq_ignore_ascii_case(b"requirement:") {
        return None;
    }
    let name = normalize(rest.get(12..).unwrap_or_default());
    (!name.is_empty()).then(|| name.to_vec())
}

/// The kind of a `##` requirement section header.
pub fn section_kind(line: &[u8]) -> Option<Kind> {
    let rest = bare(line).strip_prefix(b"##")?;
    if !rest.first().is_some_and(|&c| is_ws(c)) {
        return None;
    }
    let words: Vec<&[u8]> = rest
        .split(|&c| is_ws(c))
        .filter(|w| !w.is_empty())
        .collect();
    let req = |w: &[u8]| w.eq_ignore_ascii_case(b"requirements");
    match words.as_slice() {
        [w] if req(w) => Some(Kind::Plain),
        [op, w] if req(w) => match op.to_ascii_uppercase().as_slice() {
            b"ADDED" => Some(Kind::Added),
            b"MODIFIED" => Some(Kind::Modified),
            b"REMOVED" => Some(Kind::Removed),
            b"RENAMED" => Some(Kind::Renamed),
            _ => None,
        },
        _ => None,
    }
}

/// The ATX heading level (1–6) of a line; a requirement header is level 3.
fn level(line: &[u8]) -> Option<usize> {
    if requirement_name(line).is_some() {
        return Some(3);
    }
    let l = bare(line);
    let n = l.iter().take_while(|&&c| c == b'#').count();
    ((1..=6).contains(&n) && l.get(n).is_none_or(|&c| is_ws(c))).then_some(n)
}

fn is_blank(line: &[u8]) -> bool {
    trim_ws(bare(line)).is_empty()
}

/// `---`, `***` or `___` (three or more, spaces allowed).
fn is_break(line: &[u8]) -> bool {
    let t: Vec<u8> = bare(line).iter().copied().filter(|&c| !is_ws(c)).collect();
    t.len() >= 3 && b"-*_".iter().any(|&m| t.iter().all(|&c| c == m))
}

/// Every requirement-bearing section, fences skipped (P3c-R8). A section
/// ends at the next heading of level 1–2; a block at the next heading of
/// level 1–3, minus trailing blank lines and thematic breaks.
pub fn sections(text: &[u8]) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    let mut open: Option<Section> = None;
    let mut block: Option<(Vec<u8>, usize)> = None;
    let mut last = 0;
    let mut fences = Fences::new();
    for (s, e) in spans(text) {
        let line = text.get(s..e).unwrap_or_default();
        if fences.skip(line) {
            if block.is_some() {
                last = e;
            }
            continue;
        }
        let lvl = level(line);
        if lvl.is_some_and(|l| l <= 3)
            && let (Some(sec), Some((name, start))) = (open.as_mut(), block.take())
        {
            sec.blocks.push(Block {
                name,
                start,
                end: last,
            });
        }
        if lvl.is_some_and(|l| l <= 2) {
            if let Some(mut sec) = open.take() {
                sec.end = s;
                out.push(sec);
            }
            if let Some(kind) = section_kind(line) {
                open = Some(Section {
                    kind,
                    header: s,
                    end: text.len(),
                    blocks: Vec::new(),
                });
            }
            continue;
        }
        if open.is_none() {
            continue;
        }
        if let Some(name) = requirement_name(line) {
            block = Some((name, s));
            last = e;
        } else if block.is_some() && !is_blank(line) && !is_break(line) {
            last = e;
        }
    }
    if let (Some(sec), Some((name, start))) = (open.as_mut(), block.take()) {
        sec.blocks.push(Block {
            name,
            start,
            end: last,
        });
    }
    if let Some(sec) = open.take() {
        out.push(sec);
    }
    out
}

/// The blocks of every plain `## Requirements` section.
pub fn requirements(text: &[u8]) -> Vec<Block> {
    sections(text)
        .into_iter()
        .filter(|s| s.kind == Kind::Plain)
        .flat_map(|s| s.blocks)
        .collect()
}

/// Whether `text` has an ADDED, MODIFIED, REMOVED or RENAMED section.
pub fn has_delta(text: &[u8]) -> bool {
    sections(text).iter().any(|s| s.kind != Kind::Plain)
}

/// `text` with every byte of every `## REMOVED Requirements` section
/// dropped except its newlines, so line numbers hold (P3c-R11).
pub fn without_removed(text: &[u8]) -> Vec<u8> {
    let removed: Vec<(usize, usize)> = sections(text)
        .iter()
        .filter(|s| s.kind == Kind::Removed)
        .map(|s| (s.header, s.end))
        .collect();
    text.iter()
        .enumerate()
        .filter(|&(i, &b)| b == b'\n' || !removed.iter().any(|&(s, e)| (s..e).contains(&i)))
        .map(|(_, &b)| b)
        .collect()
}

/// The scenario names of a block: every non-fenced `####` header with a
/// leading `Scenario:` (any case) dropped, normalized like a name.
pub fn scenarios(block: &[u8]) -> Vec<Vec<u8>> {
    let mut fences = Fences::new();
    let mut out = Vec::new();
    for (s, e) in spans(block) {
        let line = block.get(s..e).unwrap_or_default();
        if fences.skip(line) || level(line) != Some(4) {
            continue;
        }
        let t = trim_ws(bare(line).get(4..).unwrap_or_default());
        let t = match t.get(..9) {
            Some(p) if p.eq_ignore_ascii_case(b"scenario:") => t.get(9..).unwrap_or_default(),
            _ => t,
        };
        out.push(normalize(t).to_vec());
    }
    out
}

/// `\r\n` when the first line of `text` ends with it, else `\n`.
fn eol(text: &[u8]) -> &'static [u8] {
    match text.iter().position(|&b| b == b'\n') {
        Some(i) if i > 0 && text.get(i - 1) == Some(&b'\r') => b"\r\n",
        _ => b"\n",
    }
}

/// A block without its final line ending, for comparisons.
fn core(b: &[u8]) -> &[u8] {
    let b = b.strip_suffix(b"\n").unwrap_or(b);
    b.strip_suffix(b"\r").unwrap_or(b)
}

fn with_eol(b: &[u8], nl: &[u8]) -> Vec<u8> {
    let mut v = b.to_vec();
    if !v.ends_with(b"\n") {
        v.extend_from_slice(nl);
    }
    v
}

/// A `**Reason**:` line after the header (the REMOVED why-line).
fn has_reason(block: &[u8]) -> bool {
    let mut fences = Fences::new();
    spans(block).iter().enumerate().any(|(i, &(s, e))| {
        let line = block.get(s..e).unwrap_or_default();
        !fences.skip(line) && i > 0 && trim_ws(bare(line)).starts_with(b"**Reason**:")
    })
}

/// `b` with every line ending turned into `nl`.
fn normalize_eol(b: &[u8], nl: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    for (i, &c) in b.iter().enumerate() {
        if c == b'\r' && b.get(i + 1) == Some(&b'\n') {
            continue;
        }
        if c == b'\n' {
            out.extend_from_slice(nl);
        } else {
            out.push(c);
        }
    }
    out
}

fn splice(doc: &[u8], start: usize, end: usize, with: &[u8]) -> Vec<u8> {
    let mut out = doc.get(..start).unwrap_or_default().to_vec();
    out.extend_from_slice(with);
    out.extend_from_slice(doc.get(end..).unwrap_or_default());
    out
}

fn find_block(doc: &[u8], name: &[u8]) -> Option<Block> {
    requirements(doc).into_iter().find(|b| b.name == name)
}

fn replace(doc: &[u8], name: &[u8], bytes: &[u8]) -> Vec<u8> {
    let Some(b) = find_block(doc, name) else {
        return doc.to_vec();
    };
    let ends_nl = doc.get(b.start..b.end).is_some_and(|o| o.ends_with(b"\n"));
    splice(
        doc,
        b.start,
        b.end,
        if ends_nl { bytes } else { core(bytes) },
    )
}

fn remove(doc: &[u8], name: &[u8]) -> Vec<u8> {
    let Some(b) = find_block(doc, name) else {
        return doc.to_vec();
    };
    let lines = spans(doc);
    let mut end = b.end;
    for &(s, e) in &lines {
        if s < b.end {
            continue;
        }
        if !is_blank(doc.get(s..e).unwrap_or_default()) {
            break;
        }
        end = e;
    }
    // At EOF, drop the gap before the block instead, so no blank tail stays.
    let mut start = b.start;
    if end == doc.len() {
        for &(s, e) in lines.iter().rev() {
            if e > b.start {
                continue;
            }
            if !is_blank(doc.get(s..e).unwrap_or_default()) {
                break;
            }
            start = s;
        }
    }
    splice(doc, start, end, b"")
}

/// Appends `bytes` after the last content line of the last plain
/// `## Requirements` section; without one, appends that section at EOF.
fn add(doc: &[u8], bytes: &[u8]) -> Vec<u8> {
    let nl = eol(doc);
    let Some(sec) = sections(doc).into_iter().rfind(|s| s.kind == Kind::Plain) else {
        let mut out = doc.to_vec();
        if !out.is_empty() {
            if !out.ends_with(b"\n") {
                out.extend_from_slice(nl);
            }
            out.extend_from_slice(nl);
        }
        out.extend_from_slice(b"## Requirements");
        out.extend_from_slice(nl);
        out.extend_from_slice(nl);
        out.extend_from_slice(bytes);
        return out;
    };
    let mut at = sec.header;
    let mut fences = Fences::new();
    for (s, e) in spans(doc) {
        if s < sec.header || s >= sec.end {
            continue;
        }
        if s == sec.header {
            at = e;
            continue;
        }
        let line = doc.get(s..e).unwrap_or_default();
        if fences.skip(line) || (!is_blank(line) && !is_break(line)) {
            at = e;
        }
    }
    let mut with = Vec::new();
    if !doc.get(..at).unwrap_or_default().ends_with(b"\n") {
        with.extend_from_slice(nl);
    }
    with.extend_from_slice(nl);
    with.extend_from_slice(bytes);
    splice(doc, at, at, &with)
}

/// Applies `delta`'s sections to `target` (P3c-R9): REMOVED, then MODIFIED,
/// then ADDED. Err holds every refusal, and then nothing is applied.
pub fn apply(target: &[u8], delta: &[u8]) -> Result<(Vec<u8>, Vec<Change>), Vec<String>> {
    let nl = eol(target);
    let mut errs: Vec<String> = Vec::new();
    let secs = sections(delta);
    if secs.iter().any(|s| s.kind == Kind::Renamed) {
        errs.push("RENAMED Requirements is not supported; write REMOVED + ADDED".to_string());
    }
    let mut entries: Vec<(Kind, Vec<u8>, Vec<u8>)> = Vec::new();
    for kind in [Kind::Removed, Kind::Modified, Kind::Added] {
        for sec in secs.iter().filter(|s| s.kind == kind) {
            for b in &sec.blocks {
                let bytes = with_eol(
                    &normalize_eol(delta.get(b.start..b.end).unwrap_or_default(), nl),
                    nl,
                );
                entries.push((kind, b.name.clone(), bytes));
            }
        }
    }
    let mut seen: Vec<&[u8]> = Vec::new();
    for (_, name, _) in &entries {
        if seen.contains(&name.as_slice()) {
            errs.push(format!(
                "duplicate requirement '{}' in the delta",
                lossy(name)
            ));
        } else {
            seen.push(name);
        }
    }
    for (kind, name, bytes) in &entries {
        if *kind == Kind::Removed && !has_reason(bytes) {
            errs.push(format!("REMOVED '{}' has no **Reason**: line", lossy(name)));
        }
    }
    let index = requirements(target);
    let mut names: Vec<&[u8]> = Vec::new();
    for b in &index {
        if names.contains(&b.name.as_slice()) {
            errs.push(format!(
                "the target has duplicate requirement '{}'",
                lossy(&b.name)
            ));
        } else {
            names.push(&b.name);
        }
    }
    let current = |name: &[u8]| {
        index
            .iter()
            .find(|b| b.name == name)
            .and_then(|b| target.get(b.start..b.end))
    };
    let mut changes = Vec::new();
    for (kind, name, bytes) in &entries {
        let applied = match (kind, current(name)) {
            (Kind::Removed, now) => now.is_some(),
            (Kind::Modified, None) => {
                errs.push(format!("MODIFIED '{}' is not in the target", lossy(name)));
                false
            }
            (Kind::Modified, Some(old)) => core(old) != core(bytes),
            (Kind::Added, None) => true,
            (Kind::Added, Some(old)) if core(old) == core(bytes) => false,
            (Kind::Added, Some(_)) => {
                errs.push(format!(
                    "ADDED '{}' already exists in the target with other text",
                    lossy(name)
                ));
                false
            }
            _ => false,
        };
        changes.push(Change {
            kind: *kind,
            name: name.clone(),
            applied,
        });
    }
    if !errs.is_empty() {
        return Err(errs);
    }
    let mut doc = target.to_vec();
    for ((kind, name, bytes), change) in entries.iter().zip(&changes) {
        if change.applied {
            doc = match kind {
                Kind::Removed => remove(&doc, name),
                Kind::Modified => replace(&doc, name, bytes),
                _ => add(&doc, bytes),
            };
        }
    }
    Ok((doc, changes))
}

/// Where a fallback feature's deltas land (P3c-R10): its product's
/// `parent:` resolved from `feature_dir` and normalized, else
/// `<root>/product.md`. Err when it leaves the spec root.
pub fn target(root: &Path, feature_dir: &Path, product: &[u8]) -> Result<PathBuf, String> {
    let parent = crate::frontmatter::parse(product)
        .and_then(|fm| fm.scalar("parent").map(|p| p.trim().to_string()))
        .filter(|p| !p.is_empty());
    let Some(parent) = parent else {
        return Ok(root.join("product.md"));
    };
    let path = lexical(&feature_dir.join(&parent));
    let base = lexical(root);
    if !path.starts_with(&base) {
        Err(format!("parent '{parent}' leaves the spec root"))
    } else if path.starts_with(base.join("features")) {
        Err(format!("parent '{parent}' points inside features/"))
    } else {
        Ok(path)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const TARGET: &str = "# Auth\n\n## Requirements\n\n### Requirement: Sign in (R1)\nUsers SHALL sign in.\n\n#### Scenario: ok\n- **WHEN** valid\n- **THEN** in\n\n### Requirement: Sign out (R2)\nUsers SHALL sign out.\n\n#### Scenario: out\n- **WHEN** asked\n- **THEN** out\n\n---\n\n## Non-Goals\n\n- SSO\n";
    const AUDIT: &str = "## ADDED Requirements\n\n### Requirement: Audit\nThe system SHALL log.\n\n#### Scenario: log\n- **WHEN** x\n- **THEN** y\n";

    fn applied(target: &str, delta: &str) -> (String, Vec<(Kind, String, bool)>) {
        let (doc, changes) = apply(target.as_bytes(), delta.as_bytes()).unwrap();
        (
            String::from_utf8(doc).unwrap(),
            changes
                .into_iter()
                .map(|c| (c.kind, String::from_utf8(c.name).unwrap(), c.applied))
                .collect(),
        )
    }

    #[test]
    fn names_follow_openspec() {
        assert_eq!(
            requirement_name(b"### Requirement: Sign in (R1)\n"),
            Some(b"Sign in (R1)".to_vec())
        );
        assert_eq!(
            requirement_name(b"###requirement:  Export  ##  \r\n"),
            Some(b"Export".to_vec())
        );
        assert_eq!(
            requirement_name(b"### Requirement: C# basics\n"),
            Some(b"C# basics".to_vec())
        );
        assert_eq!(requirement_name(b"#### Requirement: x\n"), None);
        assert_eq!(requirement_name(b"### Requirement:   \n"), None);
        assert_eq!(requirement_name(b"## Requirement: x\n"), None);
    }

    #[test]
    fn section_kinds() {
        assert_eq!(section_kind(b"## Requirements\n"), Some(Kind::Plain));
        assert_eq!(
            section_kind(b"## added   requirements \r\n"),
            Some(Kind::Added)
        );
        assert_eq!(
            section_kind(b"## RENAMED Requirements\n"),
            Some(Kind::Renamed)
        );
        assert_eq!(section_kind(b"## Requirements Trace\n"), None);
        assert_eq!(section_kind(b"### Requirements\n"), None);
        assert_eq!(section_kind(b"##Requirements\n"), None);
    }

    #[test]
    fn block_edges() {
        let text = "## Requirements\n\n### Requirement: A\nThe system SHALL a.\n\n```text\n### Requirement: Fenced\n## Not a section\n```\n\nTrailing prose.\n\n---\n\n## Next\n\n### Requirement: Outside\nx\n";
        let secs = sections(text.as_bytes());
        assert_eq!(secs.len(), 1);
        let b = &secs[0].blocks;
        assert_eq!(b.len(), 1);
        assert_eq!(
            &text.as_bytes()[b[0].start..b[0].end],
            "### Requirement: A\nThe system SHALL a.\n\n```text\n### Requirement: Fenced\n## Not a section\n```\n\nTrailing prose.\n".as_bytes()
        );
        let eof = "## Requirements\n### Requirement: B\nx";
        let s = sections(eof.as_bytes());
        assert_eq!(
            &eof.as_bytes()[s[0].blocks[0].start..s[0].blocks[0].end],
            b"### Requirement: B\nx"
        );
    }

    #[test]
    fn modified_replaces_in_place_byte_exact() {
        let delta = "## MODIFIED Requirements\n\n### Requirement: Sign in (R1)\nUsers SHALL sign in with a passkey.\n\n#### Scenario: ok\n- **WHEN** valid\n- **THEN** in\n";
        let (doc, ch) = applied(TARGET, delta);
        assert_eq!(
            doc,
            TARGET.replace(
                "Users SHALL sign in.\n",
                "Users SHALL sign in with a passkey.\n"
            )
        );
        assert_eq!(ch, [(Kind::Modified, "Sign in (R1)".to_string(), true)]);
        let last = "## MODIFIED Requirements\n### Requirement: Sign out (R2)\nUsers MUST sign out.\n#### Scenario: out\n- **WHEN** asked\n- **THEN** out\n";
        let (doc, _) = applied(TARGET, last);
        assert_eq!(
            doc,
            TARGET.replace(
                "Users SHALL sign out.\n\n#### Scenario: out",
                "Users MUST sign out.\n#### Scenario: out"
            )
        );
    }

    #[test]
    fn added_lands_after_trailing_prose_and_lists() {
        let block = "### Requirement: Audit\nThe system SHALL log.\n\n#### Scenario: log\n- **WHEN** x\n- **THEN** y\n";
        let (doc, ch) = applied(TARGET, AUDIT);
        assert_eq!(
            doc,
            TARGET.replace(
                "- **THEN** out\n\n---",
                &format!("- **THEN** out\n\n{block}\n---")
            )
        );
        assert_eq!(ch, [(Kind::Added, "Audit".to_string(), true)]);
        let list = "# P\n\n## Requirements\n\nAt a project level, P must:\n\n1. **One.** a\n2. **Two.** b\n\n---\n\n## Design Principles\n";
        let (doc, _) = applied(list, AUDIT);
        assert_eq!(
            doc,
            list.replace("2. **Two.** b\n", &format!("2. **Two.** b\n\n{block}"))
        );
        let (doc, _) = applied("# P\n\nNo section.", AUDIT);
        assert_eq!(
            doc,
            format!("# P\n\nNo section.\n\n## Requirements\n\n{block}")
        );
    }

    #[test]
    fn removed_needs_a_reason_and_removes_the_block() {
        let delta = "## REMOVED Requirements\n\n### Requirement: Sign out (R2)\n**Reason**: sessions never end.\n";
        let (doc, ch) = applied(TARGET, delta);
        assert_eq!(
            doc,
            TARGET.replace("### Requirement: Sign out (R2)\nUsers SHALL sign out.\n\n#### Scenario: out\n- **WHEN** asked\n- **THEN** out\n\n", "")
        );
        assert_eq!(ch, [(Kind::Removed, "Sign out (R2)".to_string(), true)]);
        let err = apply(
            TARGET.as_bytes(),
            b"## REMOVED Requirements\n### Requirement: Sign out (R2)\nGone.\n",
        )
        .unwrap_err();
        assert_eq!(err, ["REMOVED 'Sign out (R2)' has no **Reason**: line"]);
    }

    #[test]
    fn removing_the_last_block_at_eof_drops_the_gap() {
        let t = "## Requirements\n\n### Requirement: A\nx\n\n### Requirement: B\ny\n";
        let (doc, _) = applied(
            t,
            "## REMOVED Requirements\n### Requirement: B\n**Reason**: z\n",
        );
        assert_eq!(doc, "## Requirements\n\n### Requirement: A\nx\n");
    }

    #[test]
    fn reapplying_is_idempotent() {
        let delta = "## REMOVED Requirements\n### Requirement: Sign out (R2)\n**Reason**: x\n\n## MODIFIED Requirements\n### Requirement: Sign in (R1)\nUsers SHALL sign in fast.\n#### Scenario: ok\n- **WHEN** a\n- **THEN** b\n\n## ADDED Requirements\n### Requirement: Audit\nThe system SHALL log.\n#### Scenario: log\n- **WHEN** x\n- **THEN** y\n";
        let (once, _) = applied(TARGET, delta);
        let (twice, ch) = applied(&once, delta);
        assert_eq!(twice, once);
        assert!(ch.iter().all(|(_, _, applied)| !applied), "{ch:?}");
    }

    #[test]
    fn refusals_are_collected_and_nothing_applies() {
        let delta = "## MODIFIED Requirements\n### Requirement: Ghost\nx\n\n## ADDED Requirements\n### Requirement: Sign in (R1)\nOther text.\n\n### Requirement: Twice\na\n\n### Requirement: Twice\nb\n\n## RENAMED Requirements\n- FROM: `### Requirement: A`\n- TO: `### Requirement: B`\n";
        let err = apply(TARGET.as_bytes(), delta.as_bytes()).unwrap_err();
        assert_eq!(
            err,
            [
                "RENAMED Requirements is not supported; write REMOVED + ADDED",
                "duplicate requirement 'Twice' in the delta",
                "MODIFIED 'Ghost' is not in the target",
                "ADDED 'Sign in (R1)' already exists in the target with other text",
            ]
        );
        let dup = "## Requirements\n### Requirement: A\nx\n### Requirement: A\ny\n";
        assert_eq!(
            apply(dup.as_bytes(), AUDIT.as_bytes()).unwrap_err(),
            ["the target has duplicate requirement 'A'"]
        );
    }

    #[test]
    fn crlf_target_keeps_crlf() {
        let target = TARGET.replace('\n', "\r\n");
        let delta = "## ADDED Requirements\n### Requirement: Audit\nThe system SHALL log.\n#### Scenario: log\n- **WHEN** x\n- **THEN** y";
        let (doc, _) = applied(&target, delta);
        assert!(
            doc.contains("- **THEN** out\r\n\r\n### Requirement: Audit\r\nThe system SHALL log.\r\n#### Scenario: log\r\n- **WHEN** x\r\n- **THEN** y\r\n\r\n---"),
            "{doc:?}"
        );
        let (again, ch) = applied(&doc, delta);
        assert_eq!(again, doc);
        assert!(!ch[0].2);
    }

    #[test]
    fn lf_delta_on_crlf_target_is_a_noop_when_equal() {
        let target = TARGET.replace('\n', "\r\n");
        let modify = "## MODIFIED Requirements\n\n### Requirement: Sign in (R1)\nUsers SHALL sign in.\n\n#### Scenario: ok\n- **WHEN** valid\n- **THEN** in\n";
        let (doc, ch) = applied(&target, modify);
        assert_eq!(doc, target);
        assert_eq!(ch, [(Kind::Modified, "Sign in (R1)".to_string(), false)]);
        let add = "## ADDED Requirements\n\n### Requirement: Sign out (R2)\nUsers SHALL sign out.\n\n#### Scenario: out\n- **WHEN** asked\n- **THEN** out\n";
        let (doc, ch) = applied(&target, add);
        assert_eq!(doc, target);
        assert_eq!(ch, [(Kind::Added, "Sign out (R2)".to_string(), false)]);
    }

    #[test]
    fn fenced_reason_does_not_count() {
        let delta = "## REMOVED Requirements\n### Requirement: Sign out (R2)\n```\n**Reason**: hidden\n```\n";
        let err = apply(TARGET.as_bytes(), delta.as_bytes()).unwrap_err();
        assert_eq!(err, ["REMOVED 'Sign out (R2)' has no **Reason**: line"]);
    }

    #[test]
    fn without_removed_blanks_only_removed_sections() {
        let text = "## REMOVED Requirements\n### Requirement: X (R9)\n**Reason**: y\n\n## Requirements\n### Requirement: Y\n";
        assert_eq!(
            without_removed(text.as_bytes()),
            b"\n\n\n\n## Requirements\n### Requirement: Y\n"
        );
        assert!(has_delta(text.as_bytes()));
        assert!(!has_delta(b"## Requirements\n### Requirement: Y\n"));
    }

    #[test]
    fn scenario_names() {
        assert_eq!(
            scenarios(b"### Requirement: A\nx\n#### Scenario: One\n- a\n####  scenario:two  ##\n```\n#### Scenario: fenced\n```\n"),
            [b"One".to_vec(), b"two".to_vec()]
        );
    }

    #[test]
    fn targets_resolve_from_the_feature_folder() {
        let root = Path::new("/p/.spec");
        let fd = root.join("features").join("auth");
        assert_eq!(
            target(root, &fd, b"---\nparent: ../../product-auth.md\n---\n").unwrap(),
            root.join("product-auth.md")
        );
        assert_eq!(
            target(root, &fd, b"# no frontmatter\n").unwrap(),
            root.join("product.md")
        );
        assert_eq!(
            target(root, &fd, b"---\nparent: ../../../x.md\n---\n").unwrap_err(),
            "parent '../../../x.md' leaves the spec root"
        );
        assert_eq!(
            target(root, &fd, b"---\nparent: product.md\n---\n").unwrap_err(),
            "parent 'product.md' points inside features/"
        );
    }
}
