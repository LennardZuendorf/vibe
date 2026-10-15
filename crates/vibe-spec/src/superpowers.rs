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

/// A real calendar-shaped date: `is_date` plus month 01–12 and day 01–31.
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
    let Some(target) = first_path(line.get(9..).unwrap_or_default()) else {
        r.warn(
            "SF36",
            format!("{rel}: the '**Spec:**' line names no path").as_bytes(),
        );
        return;
    };
    let target: Vec<u8> = target
        .split(|&c| c == b'#')
        .next()
        .unwrap_or_default()
        .to_vec();
    let here = path.parent().unwrap_or(project);
    if !(join_raw(project, &target).exists() || join_raw(here, &target).exists()) {
        r.warn(
            "SF36",
            format!("{rel}: **Spec:** path '{}' does not exist", lossy(&target)).as_bytes(),
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
            let file = e.file_type().is_ok_and(|t| t.is_file());
            if file {
                r.examine(&rel);
            } else {
                r.at(&rel);
            }
            if !file || !dated(&name, suffix) {
                r.warn("SF35", format!("{rel}: not named {shape}").as_bytes());
                continue;
            }
            if dir == PLANS {
                spec_link(r, project, &rel, &e.path());
            }
        }
    }
}
