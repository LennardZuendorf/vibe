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
}
