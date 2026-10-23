//! Hand-written glob matching over `/`-separated relative paths.
//!
//! `*` matches any run of characters except `/`; `**` matches any run
//! including `/`, and `**/` may also match nothing; `?` matches one
//! character except `/`; `[abc]`, `[a-z]` and `[!abc]` (or `[^abc]`) match one
//! character except `/`; `\x` matches `x` literally. An unterminated `[` is
//! a literal. The whole path must match.
//!
//! Dialect: path-aware. A single `*` or `?` never crosses `/`. This is NOT the
//! flow write-policy dialect, where `*` does cross `/`. A port of the write
//! policy needs its own matcher, proven against the policy oracle.

/// Returns whether `path` matches `pattern` in full.
///
/// Work is bounded by `pattern length * path length`: failed positions are
/// remembered, so repeated wildcards cannot cause exponential backtracking.
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let s: Vec<char> = path.chars().collect();
    let mut failed = vec![false; (p.len() + 1) * (s.len() + 1)];
    match_at(&p, 0, &s, 0, &mut failed)
}

fn match_at(p: &[char], pi: usize, s: &[char], si: usize, failed: &mut [bool]) -> bool {
    let key = pi * (s.len() + 1) + si;
    if failed.get(key) == Some(&true) {
        return false;
    }
    let hit = step(p, pi, s, si, failed);
    if let Some(slot) = failed.get_mut(key).filter(|_| !hit) {
        *slot = true;
    }
    hit
}

fn step(p: &[char], pi: usize, s: &[char], si: usize, failed: &mut [bool]) -> bool {
    let Some(&c) = p.get(pi) else {
        return si == s.len();
    };
    match c {
        '*' if p.get(pi + 1) == Some(&'*') => {
            let rest = pi + 2;
            if p.get(rest) == Some(&'/') && match_at(p, rest + 1, s, si, failed) {
                return true;
            }
            (si..=s.len()).any(|k| match_at(p, rest, s, k, failed))
        }
        '*' => {
            let mut k = si;
            loop {
                if match_at(p, pi + 1, s, k, failed) {
                    return true;
                }
                match s.get(k) {
                    Some(&ch) if ch != '/' => k += 1,
                    _ => return false,
                }
            }
        }
        '?' => {
            matches!(s.get(si), Some(&ch) if ch != '/') && match_at(p, pi + 1, s, si + 1, failed)
        }
        '[' => match class(p, pi, s.get(si).copied()) {
            Some((hit, end)) => hit && match_at(p, end, s, si + 1, failed),
            None => s.get(si) == Some(&'[') && match_at(p, pi + 1, s, si + 1, failed),
        },
        '\\' => match p.get(pi + 1) {
            Some(&lit) => s.get(si) == Some(&lit) && match_at(p, pi + 2, s, si + 1, failed),
            None => s.get(si) == Some(&'\\') && si + 1 == s.len(),
        },
        lit => s.get(si) == Some(&lit) && match_at(p, pi + 1, s, si + 1, failed),
    }
}

/// Parses the class opening at `p[pi] == '['` against `ch`.
/// Returns `(matched, index after ']')`, or `None` when the class never closes.
fn class(p: &[char], pi: usize, ch: Option<char>) -> Option<(bool, usize)> {
    let mut i = pi + 1;
    let negate = matches!(p.get(i), Some('!' | '^'));
    if negate {
        i += 1;
    }
    let first = i;
    let mut hit = false;
    while let Some(&c) = p.get(i) {
        if c == ']' && i > first {
            let ok = matches!(ch, Some(x) if x != '/') && hit != negate;
            return Some((ok, i + 1));
        }
        match (p.get(i + 1), p.get(i + 2)) {
            (Some('-'), Some(&hi)) if hi != ']' => {
                hit |= ch.is_some_and(|x| c <= x && x <= hi);
                i += 3;
            }
            _ => {
                hit |= ch == Some(c);
                i += 1;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn table() {
        let cases: &[(&str, &str, bool)] = &[
            ("", "", true),
            ("", "a", false),
            ("*", "", true),
            ("*.md", "a.md", true),
            ("*.md", "d/a.md", false),
            ("a*", "a/b", false),
            ("**", "a/b/c", true),
            ("**/*.md", "a.md", true),
            ("**/*.md", "d/e/a.md", true),
            ("**/*.md", "d/e/a.txt", false),
            ("docs/**", "docs/a/b", true),
            ("docs/**", "docs", false),
            ("a/**/b", "a/b", true),
            ("a/**/b", "a/x/y/b", true),
            ("a/**/b", "a/x/y/c", false),
            ("?.sh", "a.sh", true),
            ("?.sh", "ab.sh", false),
            ("?", "/", false),
            ("[ab].md", "b.md", true),
            ("[ab].md", "c.md", false),
            ("[!ab].md", "c.md", true),
            ("[!ab].md", "a.md", false),
            ("[^ab].md", "a.md", false),
            ("[a-c]x", "bx", true),
            ("[a-c]x", "dx", false),
            ("[a-]x", "-x", true),
            ("[]]", "]", true),
            ("[!a]", "/", false),
            ("[", "[", true),
            ("[ab", "[ab", true),
            ("\\*", "*", true),
            ("\\*", "a", false),
            ("a\\", "a\\", true),
            ("x/y/z/*", "x/y/z/w.md", true),
            ("x/y/z/*", "x/y/z/a/w.md", false),
            ("é*", "ecole", false),
            ("é*", "étude", true),
        ];
        for &(pattern, path, want) in cases {
            assert_eq!(
                glob_match(pattern, path),
                want,
                "glob_match({pattern:?}, {path:?})"
            );
        }
    }

    #[test]
    fn hostile_patterns_finish_quickly() {
        let start = std::time::Instant::now();
        assert!(!glob_match("*a*a*a*a*a*a*b", &"a".repeat(300)));
        assert!(!glob_match(
            "**/**/**/**/**/**/**/b",
            &("a/".repeat(150) + "c")
        ));
        let elapsed = start.elapsed();
        assert!(elapsed.as_secs() < 2, "took {elapsed:?}");
    }
}
