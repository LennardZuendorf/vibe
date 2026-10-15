//! Hand-written versions of the oracles' `grep -oE`, `sed` and `awk`
//! patterns (no `regex` crate). Each function names the pattern it ports.
//! `grep -o` semantics: leftmost match, scanning resumes after it.

/// `grep -oE '\[[^]]*\]\([^)]+\)'`: markdown links in one line.
pub fn md_links(line: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < line.len() {
        if line[i] == b'['
            && let Some(end) = link_end(line, i)
        {
            out.push(&line[i..=end]);
            i = end + 1;
            continue;
        }
        i += 1;
    }
    out
}

/// The index of the `)` closing a link that starts at `line[start] == '['`.
fn link_end(line: &[u8], start: usize) -> Option<usize> {
    let close = start + 1 + line.get(start + 1..)?.iter().position(|&c| c == b']')?;
    if line.get(close + 1) != Some(&b'(') {
        return None;
    }
    let body = close + 2;
    let paren = body + line.get(body..)?.iter().position(|&c| c == b')')?;
    (paren > body).then_some(paren)
}

/// `sed 's/.*(\(.*\))/\1/' | sed 's/#.*//'` on one link: the text after
/// the last `(` up to the final `)`, cut at the first `#`.
pub fn md_link_target(link: &[u8]) -> Vec<u8> {
    let inner = match link.iter().rposition(|&c| c == b'(') {
        Some(open) if link.last() == Some(&b')') && open + 1 < link.len() => {
            &link[open + 1..link.len() - 1]
        }
        _ => link,
    };
    match inner.iter().position(|&c| c == b'#') {
        Some(hash) => inner[..hash].to_vec(),
        None => inner.to_vec(),
    }
}

/// `grep -oE '\(features/[^)]+\)|\(archive/[^)]+\)' | sed 's/^(//;s/)$//'`.
pub fn paren_links(line: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < line.len() {
        let rest = &line[i..];
        let kind = [&b"(features/"[..], b"(archive/"]
            .into_iter()
            .find(|k| rest.starts_with(k));
        if let Some(kind) = kind {
            let body = i + kind.len();
            if let Some(len) = line[body..].iter().position(|&c| c == b')')
                && len > 0
            {
                out.push(&line[i + 1..body + len]);
                i = body + len + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn digits_at(s: &[u8], i: usize) -> usize {
    s.get(i..)
        .map(|r| r.iter().take_while(|c| c.is_ascii_digit()).count())
        .unwrap_or(0)
}

/// `[a-z0-9][a-z0-9-]*/[0-9]+` at `s[i..]`: the match length.
fn unit_slash_at(s: &[u8], i: usize) -> Option<usize> {
    let first = *s.get(i)?;
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return None;
    }
    let mut j = i + 1;
    while s
        .get(j)
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
    {
        j += 1;
    }
    if s.get(j) != Some(&b'/') {
        return None;
    }
    let d = digits_at(s, j + 1);
    (d > 0).then_some(j + 1 + d - i)
}

/// `[A-Z]{1,4}[0-9]+` at `s[i..]`. A run of five or more capitals fails at
/// `i`: its shorter prefixes are followed by a capital, not a digit.
fn unit_caps_at(s: &[u8], i: usize) -> Option<usize> {
    let caps = s
        .get(i..)?
        .iter()
        .take_while(|c| c.is_ascii_uppercase())
        .count();
    if caps == 0 || caps > 4 {
        return None;
    }
    let d = digits_at(s, i + caps);
    (d > 0).then_some(caps + d)
}

/// `([a-z0-9][a-z0-9-]*/[0-9]+|[A-Z]{1,4}[0-9]+)` at `s[i..]` (the two
/// branches start with disjoint bytes, so at most one matches).
fn unit_at(s: &[u8], i: usize) -> Option<usize> {
    unit_slash_at(s, i).or_else(|| unit_caps_at(s, i))
}

/// `grep -oE '([a-z0-9][a-z0-9-]*/[0-9]+|[A-Z]{1,4}[0-9]+)'`.
pub fn unit_ids(line: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < line.len() {
        match unit_at(line, i) {
            Some(n) => {
                out.push(&line[i..i + n]);
                i += n;
            }
            None => i += 1,
        }
    }
    out
}

/// `^### ([a-z0-9][a-z0-9-]*/[0-9]+|[A-Z]{1,4}[0-9]+)`.
pub fn is_unit_heading(line: &[u8]) -> bool {
    line.starts_with(b"### ") && unit_at(line, 4).is_some()
}

/// `^\|[[:space:]]*(<unit>)[[:space:]]*\|`.
pub fn is_unit_row(line: &[u8]) -> bool {
    let spaces = |from: usize| {
        line.get(from..)
            .map(|r| r.iter().take_while(|&&c| crate::text::is_space(c)).count())
            .unwrap_or(0)
    };
    if line.first() != Some(&b'|') {
        return false;
    }
    let i = 1 + spaces(1);
    let Some(n) = unit_at(line, i) else {
        return false;
    };
    let j = i + n;
    line.get(j + spaces(j)) == Some(&b'|')
}

/// `[A-Za-z0-9_]`, the `\b` word class.
pub fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// `needle` in `s` with a non-word byte (or an end) on both sides.
pub fn has_word(s: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || s.len() < needle.len() {
        return false;
    }
    (0..=s.len() - needle.len()).any(|i| {
        s.get(i..).is_some_and(|rest| rest.starts_with(needle))
            && (i == 0 || s.get(i - 1).is_some_and(|&c| !is_word(c)))
            && s.get(i + needle.len()).is_none_or(|&c| !is_word(c))
    })
}

/// An RFC 2119 keyword in capitals as a whole word (P3b-R17).
pub fn rfc_keyword(s: &[u8]) -> bool {
    [&b"SHALL"[..], b"MUST", b"SHOULD", b"MAY"]
        .iter()
        .any(|k| has_word(s, k))
}

/// `grep -oE '\b[a-z][a-z0-9-]*/[0-9]+\b'`.
pub fn cross_feature_deps(line: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < line.len() {
        let starts = (i == 0 || !is_word(line[i - 1])) && line[i].is_ascii_lowercase();
        if starts {
            let mut j = i + 1;
            while line
                .get(j)
                .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
            {
                j += 1;
            }
            if line.get(j) == Some(&b'/') {
                let k = j + 1 + digits_at(line, j + 1);
                if k > j + 1 && !line.get(k).is_some_and(|&c| is_word(c)) {
                    out.push(&line[i..k]);
                    i = k;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// The R-IDs awk's `emit_r_ids` prints: repeated
/// `match(s, /(^|[^A-Za-z0-9_])R[0-9]+([^A-Za-z0-9_]|$)/)`, each match
/// consuming its trailing boundary byte, so `^` can match again after it.
pub fn r_ids(s: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut rest = s;
    'scan: loop {
        for p in 0..rest.len() {
            let r = if p == 0 && rest[0] == b'R' {
                0
            } else if !is_word(rest[p]) && rest.get(p + 1) == Some(&b'R') {
                p + 1
            } else {
                continue;
            };
            let d = digits_at(rest, r + 1);
            if d == 0 {
                continue;
            }
            let end = r + 1 + d;
            match rest.get(end) {
                Some(&c) if is_word(c) => continue,
                Some(_) => {
                    out.push(&rest[r..end]);
                    rest = &rest[end + 1..];
                    continue 'scan;
                }
                None => {
                    out.push(&rest[r..end]);
                    break 'scan;
                }
            }
        }
        break;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md_links_match_grep() {
        assert_eq!(
            md_links(b"[x](a) [y](b#c)"),
            vec![&b"[x](a)"[..], b"[y](b#c)"]
        );
        assert_eq!(md_links(b"[[a](b)"), vec![&b"[[a](b)"[..]]);
        assert!(md_links(b"[a]() [b] (c)").is_empty());
        assert_eq!(md_links(b"[a](b(c).md)"), vec![&b"[a](b(c)"[..]]);
    }

    #[test]
    fn md_link_target_matches_sed() {
        assert_eq!(md_link_target(b"[a](b(c).md)"), b"c).md");
        assert_eq!(md_link_target(b"[a](tech.md#top)"), b"tech.md");
        assert_eq!(md_link_target(b"[a](#top)"), b"");
    }

    #[test]
    fn paren_links_match_grep() {
        assert_eq!(
            paren_links(b"(features/a/) (archive/b) (features/) (other/c)"),
            vec![&b"features/a/"[..], b"archive/b"]
        );
    }

    #[test]
    fn unit_ids_match_grep() {
        assert_eq!(
            unit_ids(b"### auth/1 SHA256 ABCDE1 x"),
            vec![&b"auth/1"[..], b"SHA256", b"BCDE1"]
        );
        assert!(is_unit_heading(b"### vibe-flow/2 - x"));
        assert!(is_unit_heading(b"### M1 x"));
        assert!(!is_unit_heading(b"####  auth/1"));
        assert!(is_unit_row(b"|  AB12  | caps |"));
        assert!(is_unit_row(b"| auth/2 | row |"));
        assert!(!is_unit_row(b"| auth/2x | row |"));
    }

    #[test]
    fn cross_feature_deps_match_grep() {
        assert_eq!(
            cross_feature_deps(b"see a-b/2 and x/1y c/3 _d/4"),
            vec![&b"a-b/2"[..], b"c/3"]
        );
    }

    #[test]
    fn whole_words_only() {
        assert!(has_word(b"a owns b", b"owns"));
        assert!(has_word(b"**Owns**", b"Owns"));
        assert!(!has_word(b"downstream", b"owns"));
        assert!(!has_word(b"ownsx", b"owns"));
        assert!(has_word(b"x does not own", b"does not own"));
        assert!(rfc_keyword(b"Users MUST NOT x"));
        assert!(rfc_keyword(b"(SHALL)"));
        assert!(!rfc_keyword(b"Mayor of SHALLOW"));
        assert!(!rfc_keyword(b"users must x"));
        assert!(!rfc_keyword(b"MUSTN"));
        assert!(!has_word(b"abc", b""));
        assert!(!has_word(b"ab", b"abc"));
        assert!(has_word(b"owns", b"owns"));
    }

    #[test]
    fn r_ids_match_awk() {
        assert_eq!(
            r_ids(b"R1,R2 (R10) xR3 R4x R5"),
            vec![&b"R1"[..], b"R2", b"R10", b"R5"]
        );
        assert_eq!(
            r_ids(b"### Requirement: Sign in (R1)\nUsers"),
            vec![&b"R1"[..]]
        );
    }
}
