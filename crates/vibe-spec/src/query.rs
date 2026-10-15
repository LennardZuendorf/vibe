//! `vibe-spec plan --feature <f>` and `vibe-spec feature <f>`: the Spec
//! JSON v1 `plan` and `feature` queries (contracts/spec-json.schema.json).
//!
//! superpowers artifacts come first (D45): the newest
//! `docs/superpowers/plans/YYYY-MM-DD-<f>.md` and
//! `docs/superpowers/specs/YYYY-MM-DD-<f>-design.md` under the project.
//! Without them, the fallback folder `<root>/features/<f>/` answers.
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_core::exit;

use crate::root::{Miss, Root};
use crate::superpowers::{PLANS, SPECS};
use crate::text::{Fences, is_date, lines, lossy, trim_space, trim_start_space};

/// Fallback feature docs, in report order (`doc.kind`).
const FALLBACK_DOCS: [&str; 5] = ["product", "tech", "design", "plan", "research"];

struct Row {
    id: String,
    status: String,
    title: Option<String>,
}

/// `^[a-z0-9][a-z0-9-]*$` (`feature-name` in the schema).
fn valid_name(name: &str) -> bool {
    let b = name.as_bytes();
    b.first()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

fn checked<'a>(
    found: &'a Result<Root, Miss>,
    cmd: &str,
    name: &str,
) -> Result<&'a Root, (u8, Vec<u8>)> {
    if !valid_name(name) {
        eprintln!("vibe-spec {cmd}: invalid feature name {name:?} (lowercase-kebab)");
        return Err((exit::USAGE, Vec::new()));
    }
    found.as_ref().map_err(|miss| {
        eprintln!("{}", miss.message());
        (miss.code(), Vec::new())
    })
}

/// The newest `YYYY-MM-DD-<feature><suffix>` file in `dir`.
fn newest(dir: &Path, feature: &str, suffix: &str) -> Option<PathBuf> {
    let want = format!("-{feature}{suffix}");
    let mut hits: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| {
            let b = n.as_bytes();
            b.len() == 10 + want.len() && is_date(&b[..10]) && &b[10..] == want.as_bytes()
        })
        .collect();
    hits.sort();
    hits.pop().map(|n| dir.join(n))
}

/// `path` relative to `project`, `/`-joined; the absolute path when it lies
/// outside `project`.
fn rel(project: &Path, path: &Path) -> String {
    match path.strip_prefix(project) {
        Ok(p) if !p.as_os_str().is_empty() => p
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        _ => path.to_string_lossy().into_owned(),
    }
}

/// Read `path`, or report `vibe-spec: <path>: <error>` and exit 1.
fn read_plan(path: &Path) -> Result<Vec<u8>, (u8, Vec<u8>)> {
    std::fs::read(path).map_err(|e| {
        eprintln!("vibe-spec: {}: {e}", path.display());
        (exit::FINDINGS, Vec::new())
    })
}

/// `vibe-spec plan --feature <f> [--json]`.
pub fn plan(found: &Result<Root, Miss>, feature: &str, json_out: bool) -> (u8, Vec<u8>) {
    let root = match checked(found, "plan", feature) {
        Ok(root) => root,
        Err(e) => return e,
    };
    let (path, rows) = match newest(&root.project.join(PLANS), feature, ".md") {
        Some(p) => {
            let rows = match read_plan(&p) {
                Ok(text) => task_rows(&text),
                Err(e) => return e,
            };
            (p, rows)
        }
        None => {
            let p = root.dir.join("features").join(feature).join("plan.md");
            if !p.is_file() {
                eprintln!("vibe-spec plan: no plan for feature {feature}");
                return (exit::FINDINGS, Vec::new());
            }
            let rows = match read_plan(&p) {
                Ok(text) => table_rows(&text),
                Err(e) => return e,
            };
            (p, rows)
        }
    };
    if !json_out {
        let mut out = String::new();
        for r in &rows {
            out.push_str(&format!("{}\t{}", r.id, r.status));
            if let Some(t) = &r.title {
                out.push_str(&format!("\t{t}"));
            }
            out.push('\n');
        }
        return (exit::OK, out.into_bytes());
    }
    let rows: Vec<Value> = rows
        .iter()
        .map(|r| {
            let mut v = json!({ "id": r.id, "status": r.status });
            if let Some(t) = &r.title {
                v["title"] = json!(t);
            }
            v
        })
        .collect();
    let v = json!({
        "v": 1,
        "feature": feature,
        "rows": rows,
        "source": { "kind": "plan", "path": rel(&root.project, &path) },
    });
    (exit::OK, format!("{v}\n").into_bytes())
}

/// Rows of a superpowers plan: one per `### Task <n>[: title]`, status from
/// its checkboxes up to the next `##`/`###` heading.
fn task_rows(text: &[u8]) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let mut counts: Option<(usize, usize)> = None;
    let mut fences = Fences::new();
    for line in lines(text) {
        if fences.skip(line) {
            continue;
        }
        if line.starts_with(b"## ") || line.starts_with(b"### ") {
            close(&mut rows, &mut counts);
            if let Some((id, title)) = task_heading(line) {
                rows.push(Row {
                    id,
                    status: String::new(),
                    title,
                });
                counts = Some((0, 0));
            }
            continue;
        }
        if let Some((done, open)) = counts.as_mut() {
            let t = trim_start_space(line);
            if t.starts_with(b"- [x]") || t.starts_with(b"- [X]") {
                *done += 1;
            } else if t.starts_with(b"- [ ]") {
                *open += 1;
            }
        }
    }
    close(&mut rows, &mut counts);
    rows
}

fn close(rows: &mut [Row], counts: &mut Option<(usize, usize)>) {
    if let (Some((done, open)), Some(last)) = (counts.take(), rows.last_mut()) {
        last.status = match (done, open) {
            (0, _) => "todo",
            (_, 0) => "done",
            _ => "in-progress",
        }
        .to_string();
    }
}

/// `### Task <n>[: <title>]` → (`Task <n>`, title).
fn task_heading(line: &[u8]) -> Option<(String, Option<String>)> {
    let rest = line.strip_prefix(b"### Task ")?;
    let n = rest.iter().take_while(|c| c.is_ascii_digit()).count();
    if n == 0 {
        return None;
    }
    let after = rest.get(n..).unwrap_or_default();
    if !(after.is_empty() || after.first() == Some(&b':')) {
        return None;
    }
    let title = trim_space(after.strip_prefix(b":").unwrap_or(after));
    Some((
        format!("Task {}", lossy(&rest[..n])),
        (!title.is_empty()).then(|| lossy(title)),
    ))
}

/// The trimmed cells of a `| a | b |` row; empty for a non-table line.
fn cells(line: &[u8]) -> Vec<&[u8]> {
    let t = trim_space(line);
    let Some(inner) = t.strip_prefix(b"|") else {
        return Vec::new();
    };
    let inner = inner.strip_suffix(b"|").unwrap_or(inner);
    inner.split(|&c| c == b'|').map(trim_space).collect()
}

/// Rows of a fallback feature plan: the `| Unit | Status |` table.
fn table_rows(text: &[u8]) -> Vec<Row> {
    let ls = lines(text);
    let Some(head) = ls.iter().position(|l| {
        let c = cells(l);
        c.first() == Some(&&b"Unit"[..]) && c.get(1) == Some(&&b"Status"[..])
    }) else {
        return Vec::new();
    };
    ls.get(head + 2..)
        .unwrap_or_default()
        .iter()
        .take_while(|l| trim_start_space(l).starts_with(b"|"))
        .filter_map(|l| {
            let c = cells(l);
            let (id, status) = (c.first()?, c.get(1)?);
            (!id.is_empty() && !status.is_empty()).then(|| Row {
                id: lossy(id),
                status: lossy(status),
                title: None,
            })
        })
        .collect()
}

/// `vibe-spec feature <f> [--json]`.
pub fn feature(found: &Result<Root, Miss>, name: &str, json_out: bool) -> (u8, Vec<u8>) {
    let root = match checked(found, "feature", name) {
        Ok(root) => root,
        Err(e) => return e,
    };
    let mut docs: Vec<(&str, PathBuf)> = Vec::new();
    if let Some(p) = newest(&root.project.join(SPECS), name, "-design.md") {
        docs.push(("design", p));
    }
    if let Some(p) = newest(&root.project.join(PLANS), name, ".md") {
        docs.push(("plan", p));
    }
    let layer = if docs.is_empty() {
        let dir = root.dir.join("features").join(name);
        if !dir.is_dir() {
            eprintln!("vibe-spec feature: no feature {name}");
            return (exit::FINDINGS, Vec::new());
        }
        for kind in FALLBACK_DOCS {
            let p = dir.join(format!("{kind}.md"));
            if p.is_file() {
                docs.push((kind, p));
            }
        }
        "fallback"
    } else {
        "superpowers"
    };
    if !json_out {
        let mut out = format!("layer: {layer}\n");
        for (kind, p) in &docs {
            out.push_str(&format!("{kind}\t{}\n", rel(&root.project, p)));
        }
        return (exit::OK, out.into_bytes());
    }
    let docs: Vec<Value> = docs
        .iter()
        .map(|(kind, p)| json!({ "kind": kind, "path": rel(&root.project, p) }))
        .collect();
    let v = json!({ "v": 1, "feature": name, "layer": layer, "docs": docs });
    (exit::OK, format!("{v}\n").into_bytes())
}
