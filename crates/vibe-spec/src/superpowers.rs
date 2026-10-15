//! Superpowers artifacts (D45, P3b-R11): SF35 names under
//! `docs/superpowers/{specs,plans}/`, SF36 each plan's `**Spec:**` path.
//! Content format is not linted.
use std::path::Path;

use crate::pattern::{md_link_target, md_links};
use crate::report::Report;
use crate::text::{Fences, is_date, join_raw, lines, lossy};

/// Design specs, `YYYY-MM-DD-<topic>-design.md`.
pub const SPECS: &str = "docs/superpowers/specs";
/// Plans, `YYYY-MM-DD-<topic>.md`.
pub const PLANS: &str = "docs/superpowers/plans";

fn kebab(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|w| {
            !w.is_empty()
                && w.bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// A date-shaped name prefix: `is_date` plus a range check (month 01–12,
/// day 01–31). It is not a calendar check: `2026-02-31` passes.
fn plausible(date: &[u8]) -> bool {
    let num = |r: std::ops::Range<usize>| {
        date.get(r)
            .and_then(|b| std::str::from_utf8(b).ok())
            .and_then(|s| s.parse::<u32>().ok())
    };
    is_date(date)
        && num(5..7).is_some_and(|m| (1..=12).contains(&m))
        && num(8..10).is_some_and(|d| (1..=31).contains(&d))
}

/// `YYYY-MM-DD-<topic><suffix>`.
fn dated(name: &str, suffix: &str) -> bool {
    let Some(stem) = name.strip_suffix(suffix) else {
        return false;
    };
    let b = stem.as_bytes();
    b.len() > 11
        && plausible(b.get(..10).unwrap_or_default())
        && b.get(10) == Some(&b'-')
        && kebab(stem.get(11..).unwrap_or_default())
}

/// The first `` `…` `` span or markdown link target on a line, whichever comes first.
fn first_path(line: &[u8]) -> Option<Vec<u8>> {
    let tick = line.iter().position(|&c| c == b'`').and_then(|open| {
        let rest = line.get(open + 1..)?;
        let close = rest.iter().position(|&c| c == b'`')?;
        Some((open, rest.get(..close)?.to_vec()))
    });
    let link = md_links(line).first().map(|l| {
        let at = line
            .windows(l.len())
            .position(|w| w == *l)
            .unwrap_or(usize::MAX);
        (at, md_link_target(l))
    });
    match (tick, link) {
        (Some(t), Some(l)) => Some(if t.0 <= l.0 { t.1 } else { l.1 }),
        (Some(t), None) => Some(t.1),
        (None, Some(l)) => Some(l.1),
        (None, None) => None,
    }
    .filter(|p| !p.is_empty())
}

const NO_PATH: &str =
    "the '**Spec:**' line names no path \u{2014} put the path in backticks or a link";

/// A link target without `<…>` wrapping and without a `#anchor`.
fn clean_target(raw: &[u8]) -> Vec<u8> {
    let inner = raw
        .strip_prefix(b"<")
        .and_then(|r| r.strip_suffix(b">"))
        .unwrap_or(raw);
    inner
        .split(|&c| c == b'#')
        .next()
        .unwrap_or_default()
        .to_vec()
}

fn spec_link(r: &mut Report, project: &Path, rel: &str, path: &Path) {
    let text = std::fs::read(path).unwrap_or_default();
    let mut fences = Fences::new();
    let spec_line = lines(&text)
        .into_iter()
        .find(|l| !fences.skip(l) && l.starts_with(b"**Spec:**"));
    let Some(line) = spec_line else {
        r.warn(
            "SF36",
            format!("{rel}: no '**Spec:**' line naming the design it implements").as_bytes(),
        );
        return;
    };
    let target = first_path(line.get(9..).unwrap_or_default())
        .map(|t| clean_target(&t))
        .filter(|t| !t.is_empty());
    let Some(target) = target else {
        r.warn("SF36", format!("{rel}: {NO_PATH}").as_bytes());
        return;
    };
    let here = path.parent().unwrap_or(project);
    let candidates = if target.first() == Some(&b'/') {
        vec![join_raw(Path::new(""), &target)]
    } else {
        vec![join_raw(project, &target), join_raw(here, &target)]
    };
    let shown = lossy(&target);
    let Some(found) = candidates.iter().find(|c| c.exists()) else {
        r.warn(
            "SF36",
            format!("{rel}: **Spec:** path '{shown}' does not exist").as_bytes(),
        );
        return;
    };
    let inside = match (found.canonicalize(), project.canonicalize()) {
        (Ok(f), Ok(p)) => f.starts_with(p),
        _ => false,
    };
    if !inside {
        r.warn(
            "SF36",
            format!("{rel}: **Spec:** path '{shown}' is outside the project").as_bytes(),
        );
    } else if !found.is_file() {
        r.warn(
            "SF36",
            format!("{rel}: **Spec:** path '{shown}' is not a file").as_bytes(),
        );
    }
}

/// SF35 and SF36 when the artifact directories exist.
pub fn run(r: &mut Report, project: &Path) {
    for (dir, suffix, shape) in [
        (SPECS, "-design.md", "YYYY-MM-DD-<topic>-design.md"),
        (PLANS, ".md", "YYYY-MM-DD-<topic>.md"),
    ] {
        let Ok(read) = std::fs::read_dir(project.join(dir)) else {
            continue;
        };
        let mut entries: Vec<_> = read.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let rel = format!("{dir}/{name}");
            let file = std::fs::metadata(e.path()).is_ok_and(|m| m.is_file());
            if file {
                r.examine(&rel);
            } else {
                r.at(&rel);
                r.warn(
                    "SF35",
                    format!("{rel}: not a file (subfolders are not read)").as_bytes(),
                );
                continue;
            }
            if !dated(&name, suffix) {
                r.warn("SF35", format!("{rel}: not named {shape}").as_bytes());
                continue;
            }
            if dir == PLANS {
                spec_link(r, project, &rel, &e.path());
            }
        }
    }
}
