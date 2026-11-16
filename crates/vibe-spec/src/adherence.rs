//! F17 adherence checks over the tree (P3b-R11): SF26 frontmatter values,
//! SF27 children and backlinks, SF28 feature folders in the plan, SF29
//! freshness through git.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::frontmatter::{self, Frontmatter, Value};
use crate::git;
use crate::report::Report;
use crate::text::{
    Fences, is_date, join_raw, lines, lossy, md_files, name_of, sub_dirs, trim_space,
    trim_start_space,
};
use crate::validate::Ctx;

/// Flat-layout `type`s outside features.
pub const FLAT_TYPES: [&str; 4] = ["entrypoint", "product-topic", "tech-topic", "plan-topic"];
/// Layered-layout `type`s (D44).
pub const LAYERED_TYPES: [&str; 8] = [
    "hub", "area", "flow", "concept", "platform", "tech", "decision", "question",
];
/// Fallback feature `type`s (both layouts).
pub const FEATURE_TYPES: [&str; 5] = [
    "feature-product",
    "feature-tech",
    "feature-design",
    "feature-plan",
    "feature-research",
];
const SCOPES: [&str; 4] = ["product", "technical", "design", "implementation"];
const ENTRYPOINTS: [&str; 4] = ["product.md", "tech.md", "design.md", "plan.md"];

/// `file` relative to `dir`, `/`-joined.
pub fn rel_of(dir: &Path, file: &Path) -> String {
    file.strip_prefix(dir)
        .map(|r| {
            r.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_else(|_| file.to_string_lossy().into_owned())
}

/// Managed feature identity follows retained Markdown artifacts, not empty directories.
pub(crate) fn feature_dirs(dir: &Path, managed: bool) -> Vec<PathBuf> {
    sub_dirs(&dir.join("features"))
        .into_iter()
        .filter(|path| {
            !managed
                || crate::layered::walk(path)
                    .iter()
                    .any(|(path, _)| path.extension().is_some_and(|s| s == "md"))
        })
        .collect()
}
/// Each `features/<f>/{product,tech,design,plan,research}.md` that exists.
pub fn feature_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for fdir in sub_dirs(&dir.join("features")) {
        for doc in ["product", "tech", "design", "plan", "research"] {
            let p = fdir.join(format!("{doc}.md"));
            if p.is_file() {
                out.push(p);
            }
        }
    }
    out
}

/// The flat files SF26 reads: root `*.md` but `lessons.md`, then the feature docs.
pub fn flat_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = md_files(dir)
        .into_iter()
        .filter(|p| name_of(p) != "lessons.md")
        .collect();
    out.extend(feature_files(dir));
    out
}

/// SF26 on `files`, SF27 (flat only), SF28.
pub fn run(
    cx: &crate::api::Execution<'_>,
    r: &mut Report,
    ctx: &Ctx,
    files: &[PathBuf],
    layered: bool,
) {
    for file in files {
        let rel = rel_of(ctx.dir, file);
        r.examine(&format!("{}/{rel}", ctx.display));
        let text = std::fs::read(file).unwrap_or_default();
        if let Some(fm) = frontmatter::parse(&text) {
            values(r, &rel, &fm, layered);
        }
    }
    if !layered {
        children(r, ctx);
    }
    feature_rows(r, ctx);
    freshness(cx, r, ctx, files);
}

fn values(r: &mut Report, rel: &str, fm: &Frontmatter, layered: bool) {
    // Legacy SF20 checks the exact type of a feature's product, tech, design
    // and plan docs, so SF26's vocabulary skips them.
    let legacy_typed = rel
        .strip_prefix("features/")
        .and_then(|r| r.split_once('/'))
        .is_some_and(|(_, doc)| ["product.md", "tech.md", "design.md", "plan.md"].contains(&doc));
    match fm.get("type") {
        None => {}
        Some(Value::Scalar(t)) => {
            let t = t.as_str();
            if !legacy_typed {
                let vocabulary: Vec<&str> = if layered {
                    LAYERED_TYPES.to_vec()
                } else {
                    FLAT_TYPES.to_vec()
                }
                .into_iter()
                .chain(FEATURE_TYPES)
                .collect();
                if !vocabulary.contains(&t) {
                    r.warn(
                        "SF26",
                        format!("{rel}: type '{t}' is not one of {}", vocabulary.join(", "))
                            .as_bytes(),
                    );
                }
            }
            if t == "entrypoint"
                && let Some(s) = fm.scalar("scope")
                && !SCOPES.contains(&s)
            {
                r.warn(
                    "SF26",
                    format!("{rel}: scope '{s}' is not one of {}", SCOPES.join(", ")).as_bytes(),
                );
            }
        }
        Some(_) => r.warn(
            "SF26",
            format!("{rel}: type is blank or not a scalar value").as_bytes(),
        ),
    }
    match fm.get("updated") {
        None => {}
        Some(Value::Scalar(u)) => {
            if !is_date(u.as_bytes()) {
                r.warn(
                    "SF26",
                    format!("{rel}: updated '{u}' is not a YYYY-MM-DD date").as_bytes(),
                );
            }
        }
        Some(_) => r.warn(
            "SF26",
            format!("{rel}: updated is blank or not a scalar value").as_bytes(),
        ),
    }
}

/// A leading `./`, repeated, removed.
fn norm(mut s: &str) -> &str {
    while let Some(rest) = s.strip_prefix("./") {
        s = rest;
    }
    s
}

/// SF27. The existence check is limited to inline lists (P3b-R11); legacy
/// SF22 may also report a missing block-list child, but SF27 does not
/// depend on it.
fn children(r: &mut Report, ctx: &Ctx) {
    for name in ENTRYPOINTS {
        let Some(fm) = std::fs::read(ctx.dir.join(name))
            .ok()
            .and_then(|t| frontmatter::parse(&t))
        else {
            continue;
        };
        let inline = matches!(fm.get("children"), Some(Value::Inline(_)));
        for listed in fm.list("children") {
            let child = norm(&listed);
            if child.contains('{') {
                continue;
            }
            let path = join_raw(ctx.dir, child.as_bytes());
            if !path.is_file() {
                if inline {
                    r.at(&format!("{}/{name}", ctx.display));
                    r.warn(
                        "SF27",
                        format!(
                            "{name}: child '{child}' listed in frontmatter but file doesn't exist"
                        )
                        .as_bytes(),
                    );
                }
                continue;
            }
            let parent = std::fs::read(&path)
                .ok()
                .and_then(|t| frontmatter::parse(&t))
                .and_then(|f| f.scalar("parent").map(str::to_string));
            if let Some(p) = parent
                && norm(&p) != name
            {
                r.at(&format!("{}/{child}", ctx.display));
                r.warn(
                    "SF27",
                    format!("{child}: parent is '{p}', but {name} lists it as a child").as_bytes(),
                );
            }
        }
    }
}

/// Every table cell of `text`: `` ` `` and `*` dropped, `[text](…)` → `text`, trimmed.
fn row_cells(text: &[u8]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut fences = Fences::new();
    for line in lines(text) {
        if fences.skip(line) {
            continue;
        }
        let t = trim_start_space(line);
        if !t.starts_with(b"|") {
            continue;
        }
        for cell in t.split(|&c| c == b'|') {
            let kept: Vec<u8> = cell
                .iter()
                .copied()
                .filter(|&c| c != b'`' && c != b'*')
                .collect();
            let mut s = trim_space(&kept).to_vec();
            if s.first() == Some(&b'[')
                && let Some(end) = s.iter().position(|&c| c == b']')
            {
                s = s.get(1..end).unwrap_or_default().to_vec();
            }
            out.insert(lossy(trim_space(&s)));
        }
    }
    out
}

/// SF28: each feature folder is named by a cell of a root plan table row (P3b-R20).
fn feature_rows(r: &mut Report, ctx: &Ctx) {
    let features = feature_dirs(ctx.dir, ctx.managed);
    if features.is_empty() {
        return;
    }
    r.at(&format!("{}/plan.md", ctx.display));
    let plan = std::fs::read(ctx.dir.join("plan.md")).ok();
    let cells = plan.as_deref().map(row_cells).unwrap_or_default();
    for fdir in features {
        let f = name_of(&fdir);
        if plan.is_none() {
            r.warn(
                "SF28",
                format!("features/{f}/: no root plan.md lists it").as_bytes(),
            );
        } else if !cells.contains(&f) {
            r.warn(
                "SF28",
                format!("features/{f}/: no row in plan.md names '{f}'").as_bytes(),
            );
        }
    }
}

fn slash(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// SF29: a commit that changes a note must change its `updated:` line (P3b-R21).
///
/// The stamp cannot be compared with the last commit's date. A squash merge
/// dates the squashed commit with the *merge* day, not the day the note was
/// written, so an author who stamped honestly and merged later is reported stale
/// while nothing is wrong. Measured on shards: 31 notes stood behind their last
/// commit — 25 by one day (one commit, `#64`), 6 by 7–9 days. So the check wants
/// two signals together before it speaks. The gap must exceed the squash window
/// ([`lags`], one day), and the version before the commit must already carry the
/// same stamp: a stamp the commit moved is not stale, and neither is a stamp one
/// day old, because a next-day merge is indistinguishable from it. A note the
/// commit adds has no previous version to be stale against, so it keeps its
/// author's stamp.
///
/// Under `--changed` the same test runs against `HEAD`: the edit is not
/// committed yet, so it must move the stamp or the stamp must already name a day
/// the edit can claim — today or yesterday, the same one-day window.
///
/// Silent without git, history, a commit for the file, a `updated:` SF26 already
/// judges, or when the root lies outside the work tree.
fn freshness(cx: &crate::api::Execution<'_>, r: &mut Report, ctx: &Ctx, files: &[PathBuf]) {
    let today = crate::text::today_utc();
    let Some((top, under)) = git_top(cx, ctx) else {
        return;
    };
    let dates = git::last_touches(cx, &top, if under.is_empty() { "." } else { &under });
    // A shallow clone yields no dates: nothing to compare a committed note with.
    if dates.is_empty() && ctx.changed.is_none() {
        return;
    }
    for file in files {
        let rel = rel_of(ctx.dir, file);
        let shown = format!("{}/{rel}", ctx.display);
        let Some(path) = std::fs::canonicalize(file)
            .ok()
            .and_then(|abs| abs.strip_prefix(&top).ok().map(slash))
        else {
            continue;
        };
        let text = std::fs::read(file).unwrap_or_default();
        let Some(fm) = frontmatter::parse(&text) else {
            continue;
        };
        let Some(updated) = fm.scalar("updated") else {
            continue;
        };
        if !is_date(updated.as_bytes()) {
            continue;
        }
        let finding = if ctx.changed.is_some_and(|set| set.contains(&shown)) {
            unbumped_edit(cx, &top, &path, updated, &today)
        } else {
            match dates.get(&path) {
                Some(touch) => unbumped_commit(cx, &top, touch, &path, updated),
                None => None,
            }
        };
        if let Some(finding) = finding {
            r.at(&shown);
            r.warn("SF29", format!("{rel}: {finding}").as_bytes());
        }
    }
}

/// `updated:` behind the commit day by more than the squash window and left
/// where it was: reported unless that commit moved the stamp.
fn unbumped_commit(
    cx: &crate::api::Execution<'_>,
    top: &Path,
    touch: &git::Touch,
    path: &str,
    updated: &str,
) -> Option<String> {
    if !lags(updated, &touch.date) {
        return None;
    }
    let previous = stamp_at(cx, top, &format!("{}^", touch.sha), path);
    (previous.as_deref() == Some(updated)).then(|| {
        format!(
            "updated {updated} was not bumped by its last commit ({})",
            touch.date
        )
    })
}

/// The working tree changed this file with the stamp left where it was: reported
/// unless the stamp already names a day the edit can claim.
fn unbumped_edit(
    cx: &crate::api::Execution<'_>,
    top: &Path,
    path: &str,
    updated: &str,
    today: &str,
) -> Option<String> {
    let unchanged = stamp_at(cx, top, "HEAD", path).as_deref() == Some(updated);
    (unchanged && lags(updated, today))
        .then(|| format!("updated {updated} was not bumped by this edit"))
}

/// `date` more than one day before `limit`: the window both a next-day squash
/// merge and a local stamp against a UTC "today" can put between them.
fn lags(date: &str, limit: &str) -> bool {
    match (crate::text::days(date), crate::text::days(limit)) {
        (Some(date), Some(limit)) => date < limit - 1,
        _ => false,
    }
}

/// The `updated:` value of `path` (top-relative) at `rev`, when that revision has
/// the file and a date there.
fn stamp_at(cx: &crate::api::Execution<'_>, top: &Path, rev: &str, path: &str) -> Option<String> {
    let text = git::file_at(cx, top, rev, path)?;
    let fm = frontmatter::parse(&text)?;
    fm.scalar("updated")
        .filter(|u| is_date(u.as_bytes()))
        .map(str::to_string)
}

/// The git top and the top-relative path of the spec root, when there is one.
fn git_top(cx: &crate::api::Execution<'_>, ctx: &Ctx) -> Option<(PathBuf, String)> {
    let dir = std::fs::canonicalize(ctx.dir).ok()?;
    let top = std::fs::canonicalize(git::top(cx, &dir)?).ok()?;
    let under = slash(dir.strip_prefix(&top).ok()?);
    Some((top, under))
}
