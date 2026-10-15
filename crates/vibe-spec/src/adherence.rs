//! F17 adherence checks over the tree (P3b-R11): SF26 frontmatter values,
//! SF27 children and backlinks, SF28 feature folders in the plan. Task 8
//! adds SF29 freshness.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::frontmatter::{self, Frontmatter, Value};
use crate::report::Report;
use crate::text::{
    is_date, join_raw, lines, lossy, md_files, name_of, sub_dirs, trim_space, trim_start_space,
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
pub fn run(r: &mut Report, ctx: &Ctx, files: &[PathBuf], layered: bool) {
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
}

fn values(r: &mut Report, rel: &str, fm: &Frontmatter, layered: bool) {
    if let Some(t) = fm.scalar("type") {
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
                format!("{rel}: type '{t}' is not one of {}", vocabulary.join(", ")).as_bytes(),
            );
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
    if let Some(u) = fm.scalar("updated")
        && !is_date(u.as_bytes())
    {
        r.warn(
            "SF26",
            format!("{rel}: updated '{u}' is not a YYYY-MM-DD date").as_bytes(),
        );
    }
}

/// SF27. A missing child from a block list is legacy SF22's error, so only
/// inline lists report existence here.
fn children(r: &mut Report, ctx: &Ctx) {
    for name in ENTRYPOINTS {
        let Some(fm) = std::fs::read(ctx.dir.join(name))
            .ok()
            .and_then(|t| frontmatter::parse(&t))
        else {
            continue;
        };
        let inline = matches!(fm.get("children"), Some(Value::Inline(_)));
        for child in fm.list("children") {
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
                && p != name
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
    for line in lines(text) {
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
    let features = sub_dirs(&ctx.dir.join("features"));
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
