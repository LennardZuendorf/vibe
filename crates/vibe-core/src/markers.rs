//! Managed-block markers and the shared content hash.
use crate::{Error, Result};
use std::ops::Range;

/// Comment syntax of the file a block lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// `<!-- ... -->` lines (Markdown).
    Html,
    /// `# ...` lines (TOML and similar).
    Hash,
}

/// A located managed block.
#[derive(Debug, PartialEq, Eq)]
pub struct Block {
    /// Byte range of both marker lines including their line terminators.
    pub range: Range<usize>,
    /// Hash recorded in the begin marker.
    pub hash: String,
    /// Text strictly between the two marker lines.
    pub body: String,
}

enum Kind {
    Begin(String),
    End,
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// FNV-1a 64-bit hash of `bytes` as 16 lowercase hex digits.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut h = FNV_OFFSET;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    format!("{h:016x}")
}

/// Hash of a block body, exactly as written between the markers.
pub fn body_hash(body: &str) -> String {
    hash_bytes(body.as_bytes())
}

fn wrap(style: Style, inner: &str) -> String {
    match style {
        Style::Html => format!("<!-- {inner} -->"),
        Style::Hash => format!("# {inner}"),
    }
}

/// The begin marker line (no line terminator) for `owner` carrying `hash`.
pub fn begin_line(style: Style, owner: &str, hash: &str) -> String {
    wrap(style, &format!("vibe:{owner}:begin v=1 hash={hash}"))
}

/// The end marker line (no line terminator) for `owner`.
pub fn end_line(style: Style, owner: &str) -> String {
    wrap(style, &format!("vibe:{owner}:end"))
}

fn classify(line: &str, style: Style, owner: &str, no: usize) -> Result<Option<Kind>> {
    let l = line.strip_suffix('\n').unwrap_or(line);
    let l = l.strip_suffix('\r').unwrap_or(l);
    let (open, close) = match style {
        Style::Html => ("<!-- ", " -->"),
        Style::Hash => ("# ", ""),
    };
    let Some(inner) = l.strip_prefix(open).and_then(|s| s.strip_suffix(close)) else {
        return Ok(None);
    };
    let Some(rest) = inner
        .strip_prefix("vibe:")
        .and_then(|s| s.strip_prefix(owner))
    else {
        return Ok(None);
    };
    if rest == ":end" {
        return Ok(Some(Kind::End));
    }
    let Some(rest) = rest.strip_prefix(":begin v=") else {
        return Ok(None);
    };
    let Some((ver, hash)) = rest.split_once(" hash=") else {
        return Ok(None);
    };
    let hex = !hash.is_empty()
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !hex || ver.is_empty() || !ver.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(None);
    }
    if ver != "1" {
        return Err(Error::Invalid(format!(
            "unsupported marker version at line {no}"
        )));
    }
    Ok(Some(Kind::Begin(hash.to_string())))
}

struct Pos {
    line: usize,
    start: usize,
    end: usize,
}

/// Locate the block of `owner`; `None` when absent.
pub fn find(text: &str, style: Style, owner: &str) -> Result<Option<Block>> {
    let mut begins: Vec<(Pos, String)> = Vec::new();
    let mut ends: Vec<Pos> = Vec::new();
    let mut offset = 0;
    for (i, line) in text.split_inclusive('\n').enumerate() {
        let pos = Pos {
            line: i + 1,
            start: offset,
            end: offset + line.len(),
        };
        offset = pos.end;
        match classify(line, style, owner, i + 1)? {
            Some(Kind::Begin(h)) => begins.push((pos, h)),
            Some(Kind::End) => ends.push(pos),
            None => {}
        }
    }
    if let Some((p, _)) = begins.get(1) {
        return Err(Error::Invalid(format!(
            "duplicate begin marker at line {}",
            p.line
        )));
    }
    if let Some(p) = ends.get(1) {
        return Err(Error::Invalid(format!(
            "duplicate end marker at line {}",
            p.line
        )));
    }
    match (begins.pop(), ends.pop()) {
        (None, None) => Ok(None),
        (Some((b, _)), None) => Err(Error::Invalid(format!(
            "begin marker at line {} has no end marker",
            b.line
        ))),
        (None, Some(e)) => Err(Error::Invalid(format!(
            "end marker at line {} has no begin marker",
            e.line
        ))),
        (Some((b, hash)), Some(e)) => {
            if e.line < b.line {
                return Err(Error::Invalid(format!(
                    "end marker at line {} comes before begin marker at line {}",
                    e.line, b.line
                )));
            }
            Ok(Some(Block {
                range: b.start..e.end,
                hash,
                body: text[b.end..e.start].to_string(),
            }))
        }
    }
}

/// True when the body still hashes to the recorded hash.
pub fn intact(block: &Block) -> bool {
    body_hash(&block.body) == block.hash
}

fn eol_of(text: &str) -> &'static str {
    match text.find('\n') {
        Some(i) if text[..i].ends_with('\r') => "\r\n",
        _ => "\n",
    }
}

fn render(style: Style, owner: &str, body: &str, eol: &str) -> String {
    let mut normalized = body.replace("\r\n", "\n");
    if !normalized.is_empty() && !normalized.ends_with('\n') {
        normalized.push('\n');
    }
    let body = if eol == "\r\n" {
        normalized.replace('\n', "\r\n")
    } else {
        normalized
    };
    let mut out = begin_line(style, owner, &body_hash(&body));
    out.push_str(eol);
    out.push_str(&body);
    out.push_str(&end_line(style, owner));
    out.push_str(eol);
    out
}

/// Insert or replace the block of `owner` so its body is `body`.
pub fn upsert(text: &str, style: Style, owner: &str, body: &str) -> Result<String> {
    for (i, line) in body.split_inclusive('\n').enumerate() {
        if classify(line, style, owner, i + 1).ok().flatten().is_some() {
            return Err(Error::Invalid(format!(
                "block body line {} is a marker line for owner {owner}",
                i + 1
            )));
        }
    }
    let eol = eol_of(text);
    let mut block = render(style, owner, body, eol);
    match find(text, style, owner)? {
        Some(found) => {
            if !text[found.range.clone()].ends_with('\n') {
                block.truncate(block.len() - eol.len());
            }
            let mut out = String::with_capacity(text.len() + block.len());
            out.push_str(&text[..found.range.start]);
            out.push_str(&block);
            out.push_str(&text[found.range.end..]);
            Ok(out)
        }
        None => {
            let mut out = text.to_string();
            if !out.is_empty() {
                if !out.ends_with('\n') {
                    out.push_str(eol);
                }
                out.push_str(eol);
            }
            out.push_str(&block);
            Ok(out)
        }
    }
}

fn is_blank(line: &str) -> bool {
    line == "\n" || line == "\r\n"
}

fn line_before(text: &str, end: usize) -> Option<(usize, &str)> {
    if end == 0 {
        return None;
    }
    let start = text[..end - 1].rfind('\n').map_or(0, |i| i + 1);
    Some((start, &text[start..end]))
}

/// Remove the block of `owner` and the blank line it introduced.
pub fn remove(text: &str, style: Style, owner: &str) -> Result<String> {
    let Some(found) = find(text, style, owner)? else {
        return Ok(text.to_string());
    };
    let mut start = found.range.start;
    if let Some((blank_start, line)) = line_before(text, start)
        && is_blank(line)
    {
        let at_end = found.range.end == text.len();
        let preceded = line_before(text, blank_start).is_some_and(|(_, l)| is_blank(l));
        if at_end || preceded {
            start = blank_start;
        }
    }
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..start]);
    out.push_str(&text[found.range.end..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    const H: Style = Style::Html;

    fn err(r: Result<Option<Block>>) -> String {
        r.unwrap_err().to_string()
    }

    #[test]
    fn begin_line_html_and_hash_exact() {
        assert_eq!(
            begin_line(H, "store", "ab12"),
            "<!-- vibe:store:begin v=1 hash=ab12 -->"
        );
        assert_eq!(end_line(H, "store"), "<!-- vibe:store:end -->");
        assert_eq!(
            begin_line(Style::Hash, "store", "ab12"),
            "# vibe:store:begin v=1 hash=ab12"
        );
        assert_eq!(end_line(Style::Hash, "store"), "# vibe:store:end");
    }

    #[test]
    fn find_returns_body_hash_and_range() {
        let text = "pre\n<!-- vibe:store:begin v=1 hash=ab12 -->\nbody1\nbody2\n<!-- vibe:store:end -->\npost\n";
        let b = find(text, H, "store").unwrap().unwrap();
        assert_eq!(b.hash, "ab12");
        assert_eq!(b.body, "body1\nbody2\n");
        assert_eq!(
            &text[b.range],
            "<!-- vibe:store:begin v=1 hash=ab12 -->\nbody1\nbody2\n<!-- vibe:store:end -->\n"
        );
    }

    #[test]
    fn find_none_when_absent() {
        assert_eq!(find("hello\n", H, "store").unwrap(), None);
        assert_eq!(find("", H, "store").unwrap(), None);
    }

    #[test]
    fn find_ignores_other_owner_and_lookalikes() {
        let text = concat!(
            "<!-- vibe:other:begin v=1 hash=ab -->\n",
            "<!-- vibe:other:end -->\n",
            " <!-- vibe:store:begin v=1 hash=ab -->\n",
            "<!-- vibe:store:begin v=1 hash=ab  -->\n",
            "<!-- vibe:store:end --> \n",
            "<!-- vibe:storex:begin v=1 hash=ab -->\n",
            "<!-- vibe:storex:end -->\n",
            "text <!-- vibe:store:begin v=1 hash=ab --> text\n",
            "# vibe:store:begin v=1 hash=ab\n",
            "<!-- vibe:store:begin v=1 hash=AB -->\n",
            "<!-- vibe:store:begin v=1 hash= -->\n",
        );
        assert_eq!(find(text, H, "store").unwrap(), None);
    }

    #[test]
    fn reversed_markers_refuse() {
        let t = "a\n<!-- vibe:store:end -->\nb\n<!-- vibe:store:begin v=1 hash=ab -->\n";
        let m = err(find(t, H, "store"));
        assert!(m.contains("line 2") && m.contains("line 4"), "{m}");
    }

    #[test]
    fn duplicate_begin_refuses() {
        let t = "<!-- vibe:store:begin v=1 hash=ab -->\nx\n<!-- vibe:store:begin v=1 hash=ab -->\n<!-- vibe:store:end -->\n";
        assert!(err(find(t, H, "store")).contains("line 3"));
    }

    #[test]
    fn duplicate_end_refuses() {
        let t = "<!-- vibe:store:begin v=1 hash=ab -->\n<!-- vibe:store:end -->\nx\n<!-- vibe:store:end -->\n";
        assert!(err(find(t, H, "store")).contains("line 4"));
    }

    #[test]
    fn begin_without_end_refuses() {
        let t = "x\n<!-- vibe:store:begin v=1 hash=ab -->\ny\n";
        assert!(err(find(t, H, "store")).contains("line 2"));
    }

    #[test]
    fn end_without_begin_refuses() {
        let t = "x\ny\n<!-- vibe:store:end -->\n";
        assert!(err(find(t, H, "store")).contains("line 3"));
    }

    #[test]
    fn unsupported_version_refuses() {
        let t = "x\n<!-- vibe:store:begin v=2 hash=ab -->\n<!-- vibe:store:end -->\n";
        let m = err(find(t, H, "store"));
        assert!(
            m.contains("unsupported marker version") && m.contains("line 2"),
            "{m}"
        );
    }

    #[test]
    fn upsert_into_empty() {
        let out = upsert("", H, "store", "hi").unwrap();
        let hash = body_hash("hi\n");
        assert_eq!(
            out,
            format!("<!-- vibe:store:begin v=1 hash={hash} -->\nhi\n<!-- vibe:store:end -->\n")
        );
        let empty = upsert("", H, "store", "").unwrap();
        assert!(find(&empty, H, "store").unwrap().is_some());
    }

    #[test]
    fn upsert_appends_with_blank_line_after_text_without_trailing_newline() {
        let out = upsert("abc", H, "store", "x\n").unwrap();
        assert!(out.starts_with("abc\n\n<!-- vibe:store:begin"), "{out:?}");
        let out = upsert("abc\n", H, "store", "x\n").unwrap();
        assert!(out.starts_with("abc\n\n<!-- vibe:store:begin"), "{out:?}");
    }

    #[test]
    fn upsert_replaces_in_place_keeping_surrounding_bytes() {
        let pre = "pre \t\n\n";
        let post = "\n  post no newline";
        let first = upsert("", H, "store", "old\n").unwrap();
        let text = format!("{pre}{first}{post}");
        let out = upsert(&text, H, "store", "new\n").unwrap();
        assert!(out.starts_with(pre) && out.ends_with(post));
        let b = find(&out, H, "store").unwrap().unwrap();
        assert_eq!(b.body, "new\n");
        assert!(intact(&b));
    }

    #[test]
    fn upsert_is_idempotent() {
        let once = upsert("abc\n", H, "store", "x\ny").unwrap();
        assert_eq!(upsert(&once, H, "store", "x\ny").unwrap(), once);
        let hash = upsert("abc\n", Style::Hash, "store", "k = 1\n").unwrap();
        assert_eq!(
            upsert(&hash, Style::Hash, "store", "k = 1\n").unwrap(),
            hash
        );
    }

    #[test]
    fn crlf_file_gets_crlf_block_and_find_tolerates_cr() {
        let out = upsert("a\r\nb\r\n", H, "store", "x\ny\n").unwrap();
        assert!(
            out.ends_with("x\r\ny\r\n<!-- vibe:store:end -->\r\n"),
            "{out:?}"
        );
        assert!(!out.replace("\r\n", "").contains('\n'));
        let b = find(&out, H, "store").unwrap().unwrap();
        assert_eq!(b.body, "x\r\ny\r\n");
    }

    #[test]
    fn crlf_upsert_then_intact_is_true() {
        let out = upsert("a\r\n", H, "store", "x\ny\n").unwrap();
        assert!(intact(&find(&out, H, "store").unwrap().unwrap()));
    }

    #[test]
    fn crlf_upsert_is_idempotent() {
        let once = upsert("a\r\n", H, "store", "x\ny\n").unwrap();
        assert_eq!(upsert(&once, H, "store", "x\ny\n").unwrap(), once);
    }

    #[test]
    fn no_trailing_newline_preserved_outside_block() {
        let first = upsert("", H, "store", "x\n").unwrap();
        let first = first.trim_end_matches('\n').to_string();
        let text = format!("head\n\n{first}");
        assert!(!text.ends_with('\n'));
        let out = upsert(&text, H, "store", "y\n").unwrap();
        assert!(out.starts_with("head\n\n"));
        assert!(out.ends_with("<!-- vibe:store:end -->"), "{out:?}");
        assert!(intact(&find(&out, H, "store").unwrap().unwrap()));
    }

    #[test]
    fn remove_restores_original_text() {
        assert_eq!(
            remove(&upsert("", H, "store", "x\n").unwrap(), H, "store").unwrap(),
            ""
        );
        let t = "abc\n";
        assert_eq!(
            remove(&upsert(t, H, "store", "x\n").unwrap(), H, "store").unwrap(),
            t
        );
        let t = "abc";
        assert_eq!(
            remove(&upsert(t, H, "store", "x\n").unwrap(), H, "store").unwrap(),
            "abc\n"
        );
        let t = "a\r\nb\r\n";
        assert_eq!(
            remove(&upsert(t, H, "store", "x\n").unwrap(), H, "store").unwrap(),
            t
        );
    }

    #[test]
    fn remove_in_middle_keeps_surrounding_text() {
        let block = upsert("", H, "store", "x\n").unwrap();
        assert_eq!(
            remove(&format!("a\n{block}b\n"), H, "store").unwrap(),
            "a\nb\n"
        );
        assert_eq!(
            remove(&format!("a\n\n\n{block}b\n"), H, "store").unwrap(),
            "a\n\nb\n"
        );
    }

    #[test]
    fn remove_absent_is_identity() {
        assert_eq!(remove("abc", H, "store").unwrap(), "abc");
    }

    #[test]
    fn intact_detects_hand_edit() {
        let out = upsert("", H, "store", "hello\n").unwrap();
        assert!(intact(&find(&out, H, "store").unwrap().unwrap()));
        let edited = out.replace("hello", "hellp");
        assert!(!intact(&find(&edited, H, "store").unwrap().unwrap()));
    }

    #[test]
    fn upsert_refuses_body_holding_own_marker_line() {
        for body in [
            "a\n<!-- vibe:store:end -->\nb\n",
            "<!-- vibe:store:begin v=1 hash=ab -->\n",
            "x\r\n<!-- vibe:store:end -->\r\n",
        ] {
            assert!(matches!(
                upsert("t\n", H, "store", body),
                Err(Error::Invalid(_))
            ));
        }
        assert!(upsert("t\n", H, "store", "<!-- vibe:other:end -->\n").is_ok());
        assert!(matches!(
            upsert("", Style::Hash, "store", "# vibe:store:end"),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn upsert_and_remove_refuse_bad_markers_and_leave_input() {
        let reversed =
            "a\n<!-- vibe:store:end -->\nb\n<!-- vibe:store:begin v=1 hash=ab -->\n".to_string();
        let dup = "<!-- vibe:store:begin v=1 hash=ab -->\n<!-- vibe:store:begin v=1 hash=ab -->\n<!-- vibe:store:end -->\n".to_string();
        for text in [reversed, dup] {
            let before = text.clone();
            assert!(upsert(&text, H, "store", "x\n").is_err());
            assert!(remove(&text, H, "store").is_err());
            assert_eq!(text, before);
        }
    }

    #[test]
    fn hash_is_fnv1a64() {
        assert_eq!(body_hash(""), "cbf29ce484222325");
        assert_eq!(body_hash("a"), "af63dc4c8601ec8c");
        assert_eq!(hash_bytes(b"a"), "af63dc4c8601ec8c");
    }
}
