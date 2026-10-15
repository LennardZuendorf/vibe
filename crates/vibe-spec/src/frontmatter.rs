//! A YAML-subset frontmatter reader for the F17 checks: `key: value`
//! scalars, inline lists `[a, b]`, block lists (`key:` then `- item`
//! lines). Nested maps, comments and anything else are skipped. Values are
//! lossy UTF-8 with surrounding quotes removed.
use crate::text::{is_space, lines, lossy, trim_space, trim_start_space};

/// One frontmatter value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// `key: value`.
    Scalar(String),
    /// `key: [a, b]`.
    Inline(Vec<String>),
    /// `key:` followed by zero or more `- item` lines.
    Block(Vec<String>),
}

/// Parsed frontmatter, keys in file order.
#[derive(Debug, Default)]
pub struct Frontmatter {
    entries: Vec<(String, Value)>,
}

impl Frontmatter {
    /// The value of `key`.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// A scalar value.
    pub fn scalar(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Value::Scalar(s) => Some(s),
            _ => None,
        }
    }

    /// A list's items; a non-empty scalar is a one-item list.
    pub fn list(&self, key: &str) -> Vec<String> {
        match self.get(key) {
            Some(Value::Inline(v) | Value::Block(v)) => v.clone(),
            Some(Value::Scalar(s)) if !s.is_empty() => vec![s.clone()],
            _ => Vec::new(),
        }
    }
}

fn unquote(s: &[u8]) -> String {
    let t = trim_space(s);
    let quoted = t.len() >= 2
        && matches!(
            (t.first(), t.last()),
            (Some(b'"'), Some(b'"')) | (Some(b'\''), Some(b'\''))
        );
    lossy(if quoted {
        t.get(1..t.len() - 1).unwrap_or_default()
    } else {
        t
    })
}

/// The frontmatter between a first line `---` and the next `---` line
/// (`\r` and blanks around the dashes ignored). `None` without one, or when
/// it never closes.
pub fn parse(text: &[u8]) -> Option<Frontmatter> {
    let all = lines(text);
    let mut it = all.iter();
    if trim_space(it.next()?) != b"---" {
        return None;
    }
    let mut fm = Frontmatter::default();
    for raw in it {
        let line = raw.strip_suffix(b"\r").unwrap_or(raw);
        if trim_space(line) == b"---" {
            return Some(fm);
        }
        let t = trim_start_space(line);
        if let Some(rest) = t
            .strip_prefix(b"- ")
            .or_else(|| (t == b"-").then_some(&b""[..]))
        {
            if let Some((_, Value::Block(items))) = fm.entries.last_mut() {
                items.push(unquote(rest));
            }
            continue;
        }
        if line.first().is_none_or(|&c| is_space(c) || c == b'#') {
            continue;
        }
        let Some(colon) = line.iter().position(|&c| c == b':') else {
            continue;
        };
        let key = lossy(trim_space(line.get(..colon).unwrap_or_default()));
        let rest = trim_space(line.get(colon + 1..).unwrap_or_default());
        let value = if rest.is_empty() {
            Value::Block(Vec::new())
        } else if rest.len() >= 2 && rest.first() == Some(&b'[') && rest.last() == Some(&b']') {
            let inner = rest.get(1..rest.len() - 1).unwrap_or_default();
            Value::Inline(
                inner
                    .split(|&c| c == b',')
                    .map(unquote)
                    .filter(|s| !s.is_empty())
                    .collect(),
            )
        } else {
            Value::Scalar(unquote(rest))
        };
        fm.entries.push((key, value));
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn scalars_lists_and_blocks() {
        let fm = parse(b"---\r\ntype: \"entrypoint\"\r\nchildren: [a.md, 'b.md', ]\nscope: product\ncode:\n  - src/a.rs\n- src/b.rs\ntypography:\n  body: Inter\n# comment\nempty:\nupdated: 2026-01-15\n---\nbody: no\n").unwrap();
        assert_eq!(fm.scalar("type"), Some("entrypoint"));
        assert_eq!(
            fm.get("children"),
            Some(&Value::Inline(vec!["a.md".into(), "b.md".into()]))
        );
        assert_eq!(fm.list("code"), ["src/a.rs", "src/b.rs"]);
        assert_eq!(fm.get("typography"), Some(&Value::Block(Vec::new())));
        assert_eq!(fm.list("empty"), Vec::<String>::new());
        assert_eq!(fm.list("scope"), ["product"]);
        assert_eq!(fm.scalar("body"), None);
        assert_eq!(fm.scalar("updated"), Some("2026-01-15"));
    }

    #[test]
    fn no_or_unclosed_frontmatter_is_none() {
        assert!(parse(b"# title\n").is_none());
        assert!(parse(b"---\ntype: x\n").is_none());
        assert!(parse(b"").is_none());
        assert!(parse(b"---\n---\n").is_some());
    }
}
