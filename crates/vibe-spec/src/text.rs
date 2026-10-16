//! Byte-level helpers that reproduce how the bash oracles read and print
//! text. The oracles run under `LC_ALL=C`, so everything here works on
//! bytes: no Unicode case folding, byte order, byte lengths.
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Lines as `grep`, `awk` and `while read -r l || [[ -n $l ]]` see them:
/// split on `\n`; a final segment without `\n` counts when non-empty.
pub fn lines(text: &[u8]) -> Vec<&[u8]> {
    let mut out: Vec<&[u8]> = text.split(|&b| b == b'\n').collect();
    if out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out
}

/// Lines as `while IFS= read -r line` sees them: a final segment without
/// `\n` is dropped.
pub fn read_lines(text: &[u8]) -> Vec<&[u8]> {
    let mut out: Vec<&[u8]> = text.split(|&b| b == b'\n').collect();
    out.pop();
    out
}

/// `head -1`: the bytes before the first `\n`.
pub fn first_line(text: &[u8]) -> &[u8] {
    text.split(|&b| b == b'\n').next().unwrap_or_default()
}

/// `grep -q '^<prefix>'`.
pub fn has_line_starting(text: &[u8], prefix: &[u8]) -> bool {
    lines(text).iter().any(|l| l.starts_with(prefix))
}

/// The index of the first `needle` in `hay`.
pub fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// ASCII case-insensitive substring test (`grep -qi` with a literal under `LC_ALL=C`).
pub fn contains_ci(hay: &[u8], needle: &[u8]) -> bool {
    needle.is_empty()
        || hay
            .windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
}

/// `sed 's/<needle>//'`: `hay` without the first occurrence of `needle`.
pub fn remove_first(hay: &[u8], needle: &[u8]) -> Vec<u8> {
    if needle.is_empty() {
        return hay.to_vec();
    }
    match hay.windows(needle.len()).position(|w| w == needle) {
        Some(i) => [&hay[..i], &hay[i + needle.len()..]].concat(),
        None => hay.to_vec(),
    }
}

/// `${line#prefix}`: `line` without `prefix` when it starts with it.
pub fn strip_prefix_or<'a>(line: &'a [u8], prefix: &[u8]) -> &'a [u8] {
    line.strip_prefix(prefix).unwrap_or(line)
}

/// `[[:space:]]` under `LC_ALL=C`.
pub fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `s` without leading `[[:space:]]`.
pub fn trim_start_space(s: &[u8]) -> &[u8] {
    let n = s.iter().take_while(|&&b| is_space(b)).count();
    s.get(n..).unwrap_or_default()
}

/// `s` without trailing `[[:space:]]`.
pub fn trim_end_space(s: &[u8]) -> &[u8] {
    let n = s.iter().rev().take_while(|&&b| is_space(b)).count();
    s.get(..s.len() - n).unwrap_or_default()
}

/// `s` without leading or trailing `[[:space:]]`.
pub fn trim_space(s: &[u8]) -> &[u8] {
    let s = trim_start_space(s);
    let n = s.iter().rev().take_while(|&&b| is_space(b)).count();
    s.get(..s.len() - n).unwrap_or_default()
}

/// A fence line: <=3 leading spaces, then a run of >=3 backticks or tildes.
/// Returns (char, run length, rest of the line).
pub fn fence_run(line: &[u8]) -> Option<(u8, usize, &[u8])> {
    let indent = line.iter().take_while(|&&c| c == b' ').count();
    if indent > 3 {
        return None;
    }
    let body = line.get(indent..)?;
    let ch = *body.first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let n = body.iter().take_while(|&&c| c == ch).count();
    (n >= 3).then(|| (ch, n, body.get(n..).unwrap_or_default()))
}

/// Fenced-code tracking for the F17 line scanners.
#[derive(Default)]
pub struct Fences(Option<(u8, usize)>);

impl Fences {
    /// Outside any fence.
    pub fn new() -> Fences {
        Fences(None)
    }

    /// Feeds one line; true when it is a fence delimiter or inside a fence.
    pub fn skip(&mut self, line: &[u8]) -> bool {
        if let Some((ch, len)) = self.0 {
            if fence_run(line)
                .is_some_and(|(c, n, rest)| c == ch && n >= len && trim_space(rest).is_empty())
            {
                self.0 = None;
            }
            return true;
        }
        if let Some((ch, len, _)) = fence_run(line) {
            self.0 = Some((ch, len));
            return true;
        }
        false
    }
}

/// What bash's `echo -e "$msg"` prints, newline included. Escapes:
/// `\a \b \e \E \f \n \r \t \v \\`, `\0nnn` (≤3 octal digits),
/// `\xHH` (≤2 hex digits), `\uHHHH`, `\UHHHHHHHH`; `\c` ends the output
/// there, newline and all. Any other `\X` prints as is. A code point
/// ≥ 0x80 from `\u`/`\U` prints as UTF-8 (bash under a UTF-8 locale); no
/// corpus case uses one, because the oracle's bytes depend on the locale.
pub fn echo_e(msg: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(msg.len() + 1);
    let mut i = 0;
    while let Some(&b) = msg.get(i) {
        let Some(&c) = msg.get(i + 1).filter(|_| b == b'\\') else {
            out.push(b);
            i += 1;
            continue;
        };
        i += 2;
        match c {
            b'a' => out.push(0x07),
            b'b' => out.push(0x08),
            b'e' | b'E' => out.push(0x1b),
            b'f' => out.push(0x0c),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'v' => out.push(0x0b),
            b'\\' => out.push(b'\\'),
            b'c' => return out,
            b'0' => {
                let (v, n) = digits(msg.get(i..).unwrap_or_default(), 3, 8);
                out.push((v & 0xff) as u8);
                i += n;
            }
            b'x' | b'u' | b'U' => {
                let max = match c {
                    b'x' => 2,
                    b'u' => 4,
                    _ => 8,
                };
                let (v, n) = digits(msg.get(i..).unwrap_or_default(), max, 16);
                if n == 0 {
                    out.extend_from_slice(&[b'\\', c]);
                } else if c == b'x' {
                    out.push((v & 0xff) as u8);
                } else {
                    let ch = char::from_u32(v).unwrap_or(char::REPLACEMENT_CHARACTER);
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                i += n;
            }
            _ => out.extend_from_slice(&[b'\\', c]),
        }
    }
    out.push(b'\n');
    out
}

/// Up to `max` leading digits of `s` in `radix`: (value, digits consumed).
fn digits(s: &[u8], max: usize, radix: u32) -> (u32, usize) {
    let mut v: u32 = 0;
    let mut n = 0;
    for &b in s.iter().take(max) {
        let Some(d) = (b as char).to_digit(radix) else {
            break;
        };
        v = v.wrapping_mul(radix).wrapping_add(d);
        n += 1;
    }
    (v, n)
}

/// `bytes` as a JSON string literal, quotes included. Invalid UTF-8 becomes U+FFFD.
pub fn json_str(bytes: &[u8]) -> Vec<u8> {
    serde_json::to_string(&lossy(bytes))
        .unwrap_or_else(|_| "\"\"".to_string())
        .into_bytes()
}

/// `bytes` as a `String`, invalid UTF-8 replaced.
pub fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `"$dir/$rel"` as text: `rel` is appended after a `/`, never resolved, so
/// an absolute `rel` does not replace `dir` (unlike `Path::join`).
pub fn join_raw(dir: &Path, rel: &[u8]) -> PathBuf {
    let mut s: OsString = dir.as_os_str().to_os_string();
    s.push("/");
    s.push(os_from_bytes(rel));
    PathBuf::from(s)
}

#[cfg(unix)]
fn os_from_bytes(b: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::OsStr::from_bytes(b).to_os_string()
}

#[cfg(not(unix))]
fn os_from_bytes(b: &[u8]) -> OsString {
    OsString::from(lossy(b))
}

/// The last path component as text.
pub fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn sorted_entries(dir: &Path, keep: impl Fn(&Path, &str) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = name_of(p);
            !name.starts_with('.') && keep(p, &name)
        })
        .collect();
    out.sort_by(|a, b| {
        a.as_os_str()
            .as_encoded_bytes()
            .cmp(b.as_os_str().as_encoded_bytes())
    });
    out
}

/// `"$dir"/*.md` under nullglob: every non-hidden entry ending in .md, any
/// type, in byte order; callers read with unwrap_or_default, so an unreadable
/// entry behaves like the oracle's silenced grep/head.
pub fn md_files(dir: &Path) -> Vec<PathBuf> {
    sorted_entries(dir, |_, name| name.ends_with(".md"))
}

/// `"$dir"/*/`: subdirectories (symlinks followed). Bash sorts the expanded
/// words with their trailing `/`, so `spec-delta/` precedes `spec/`.
pub fn sub_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut out = sorted_entries(dir, |p, _| p.is_dir());
    out.sort_by_key(|p| {
        let mut key = p.as_os_str().as_encoded_bytes().to_vec();
        key.push(b'/');
        key
    });
    out
}

/// `YYYY-MM-DD` (digits and dashes only; the calendar is not checked).
pub fn is_date(b: &[u8]) -> bool {
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        })
}

/// Regular files under `dir`, recursive, as `/`-joined paths relative to
/// `dir`, sorted by bytes. Dot names are skipped; symlinks are not followed
/// and not listed.
pub fn walk_files(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let Ok(meta) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            let rel = format!("{prefix}{name}");
            if meta.is_dir() {
                walk(&entry.path(), &format!("{rel}/"), out);
            } else if meta.is_file() {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn sub_dirs_sort_with_the_trailing_slash() {
        let dir = std::env::temp_dir().join(format!("vibe-subdirs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for n in ["spec", "spec-delta", "a", "a-b", "a_b"] {
            std::fs::create_dir_all(dir.join(n)).unwrap();
        }
        let names: Vec<String> = sub_dirs(&dir).iter().map(|p| name_of(p)).collect();
        assert_eq!(names, ["a-b", "a", "a_b", "spec-delta", "spec"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lines_keep_a_final_unterminated_line_and_read_lines_drop_it() {
        assert_eq!(lines(b"a\nb"), vec![&b"a"[..], b"b"]);
        assert_eq!(lines(b"a\n\n"), vec![&b"a"[..], b""]);
        assert!(lines(b"").is_empty());
        assert_eq!(read_lines(b"a\nb"), vec![&b"a"[..]]);
        assert_eq!(read_lines(b"a\nb\n"), vec![&b"a"[..], b"b"]);
        assert!(read_lines(b"").is_empty());
    }

    #[test]
    fn echo_e_interprets_like_bash() {
        assert_eq!(
            echo_e(b"\\033[31m  ERROR: x\\033[0m"),
            b"\x1b[31m  ERROR: x\x1b[0m\n"
        );
        assert_eq!(echo_e(b"a\\tb\\x41\\\\"), b"a\tbA\\\n");
        assert_eq!(echo_e(b"keep \\q and \\"), b"keep \\q and \\\n");
        assert_eq!(echo_e(b"stop\\chere"), b"stop");
        assert_eq!(echo_e(b"\\xZZ"), b"\\xZZ\n");
    }

    #[test]
    fn join_raw_never_replaces_the_base() {
        assert_eq!(
            join_raw(Path::new(".spec"), b"/x.md"),
            PathBuf::from(".spec//x.md")
        );
        assert_eq!(
            join_raw(Path::new(".spec"), b"a.md"),
            PathBuf::from(".spec/a.md")
        );
    }

    #[test]
    fn substring_helpers_follow_grep_and_sed() {
        assert!(contains_ci(b"**Tags:** Bash, Quoting", b"bash"));
        assert!(contains_ci(b"", b""));
        assert!(!contains_ci(b"bash", b"b.sh"));
        assert_eq!(remove_first(b"type: type: x", b"type: "), b"type: x");
        assert_eq!(remove_first(b"type:x", b"type: "), b"type:x");
        assert_eq!(trim_space(b" \t x y \r"), b"x y");
    }
}
