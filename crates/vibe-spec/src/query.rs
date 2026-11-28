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

use crate::root::{FeatureLayer, Miss, Root};
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
pub(crate) fn valid_name(name: &str) -> bool {
    let b = name.as_bytes();
    b.first()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

fn checked<'a>(
    cx: &crate::api::Execution<'_>,
    found: &'a Result<Root, Miss>,
    cmd: &str,
    name: &str,
) -> Result<&'a Root, (u8, Vec<u8>)> {
    if !valid_name(name) {
        diag!(
            cx,
            "vibe-spec {cmd}: invalid feature name {name:?} (lowercase-kebab)"
        );
        return Err((exit::USAGE, Vec::new()));
    }
    found.as_ref().map_err(|miss| {
        diag!(cx, "{}", miss.message());
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

/// The superpowers artifacts bound to `<name>`, in report order.
fn superpowers_docs(project: &Path, name: &str) -> Vec<(&'static str, PathBuf)> {
    let mut docs = Vec::new();
    if let Some(p) = newest(&project.join(SPECS), name, "-design.md") {
        docs.push(("design", p));
    }
    if let Some(p) = newest(&project.join(PLANS), name, ".md") {
        docs.push(("plan", p));
    }
    docs
}

/// `path` relative to `project`, `/`-joined; the absolute path when it lies
/// outside `project`.
pub(crate) fn rel(project: &Path, path: &Path) -> String {
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
fn read_plan(cx: &crate::api::Execution<'_>, path: &Path) -> Result<Vec<u8>, (u8, Vec<u8>)> {
    std::fs::read(path).map_err(|e| {
        diag!(cx, "vibe-spec: {}: {e}", path.display());
        (exit::FINDINGS, Vec::new())
    })
}

/// The layer `.vibe/spec.json` forces, for a CLI read; a bad value is a usage
/// error naming the allowed ones.
fn forced_layer(
    cx: &crate::api::Execution<'_>,
    cmd: &str,
    project: &Path,
) -> Result<Option<FeatureLayer>, (u8, Vec<u8>)> {
    crate::root::feature_layer(project).map_err(|e| {
        diag!(cx, "vibe-spec {cmd}: {e}");
        (exit::USAGE, Vec::new())
    })
}

/// The refusal for a forced layer that holds nothing to answer: names the
/// selected layer, what it lacks, and what the other layer holds. A selected
/// backend never switches.
fn layer_refusal(cmd: &str, forced: FeatureLayer, missing: &str, other: Option<String>) -> String {
    let mut text = format!(
        "vibe-spec {cmd}: `.vibe/spec.json` sets `featureLayer` \"{}\" and the {} layer has no {missing}",
        forced.key(),
        forced.key()
    );
    if let Some(other) = other {
        text.push_str(&format!(
            "; {other} exists, and a selected layer never switches"
        ));
    }
    text
}

/// What one `plan`/`feature` lookup asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Query {
    Plan,
    Feature,
}

/// How one lookup spells the roots it searches: the superpowers tree, the spec
/// root and the openspec tree. The CLI spells them from the project; the typed
/// API uses the logical `superpowers/`, `spec/`, `openspec/` prefixes. One
/// spelling per lookup keeps refusals and misses identical across both paths.
pub(crate) struct Roots {
    /// The superpowers tree holding `plans/` and `specs/`.
    pub superpowers: String,
    /// The spec root holding `features/<f>/`.
    pub spec: String,
    /// The openspec tree holding `changes/<f>/`.
    pub openspec: String,
}

impl Roots {
    /// The CLI spelling: `project`-relative when inside the project.
    fn of(project: &Path, root: &Root) -> Self {
        let rel_root = |p: &str| rel(project, &project.join(p));
        Self {
            superpowers: rel_root(crate::superpowers::ROOT),
            spec: root.display(),
            openspec: rel_root("openspec"),
        }
    }

    /// The typed API spelling: the logical document prefixes `Files` uses.
    fn logical() -> Self {
        Self {
            superpowers: "superpowers".into(),
            spec: "spec".into(),
            openspec: "openspec".into(),
        }
    }
}

/// One feature lookup: the bound superpowers artifacts, the fallback artifact,
/// and whether the openspec backend holds the name. The caller decides what the
/// selected layer allows it to see; this type decides what that means.
pub(crate) struct Lookup<'a> {
    pub query: Query,
    pub name: &'a str,
    pub roots: Roots,
    /// Bound superpowers artifacts, displayed. Empty when the selected layer
    /// keeps the superpowers candidates out.
    pub superpowers: Vec<String>,
    /// The fallback artifact or folder, displayed, when it holds the name.
    pub fallback: Option<String>,
    /// `openspec/changes/<f>`, displayed, when that directory exists.
    pub openspec: Option<String>,
}

/// Why a lookup must refuse before any artifact is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// Two backends hold one name: an error, not a precedence question.
    Ambiguous,
    /// Only openspec holds it, and this build has no openspec reader (#107).
    OpenspecOnly,
}

impl Refusal {
    /// The typed API error code for this refusal.
    pub fn code(self) -> &'static str {
        match self {
            Refusal::Ambiguous => "feature_ambiguous",
            Refusal::OpenspecOnly => "openspec_unreadable",
        }
    }
}

impl Lookup<'_> {
    /// The refusal this lookup owes, or `None` when one candidate may answer.
    /// A forced layer only changes what `superpowers` holds; a second backend
    /// holding the name stays a fact about the tree, never a preference.
    pub fn refusal(&self) -> Option<Refusal> {
        self.openspec.as_ref()?;
        Some(if self.superpowers.is_empty() && self.fallback.is_none() {
            Refusal::OpenspecOnly
        } else {
            Refusal::Ambiguous
        })
    }

    /// The refusal, one line, naming both candidates.
    pub fn refusal_message(&self, refusal: Refusal) -> String {
        let name = self.name;
        match refusal {
            Refusal::Ambiguous => {
                let mut held = self.superpowers.clone();
                held.extend(self.fallback.iter().cloned());
                format!(
                    "{name} is held by both backends: superpowers ({}) and openspec ({}); \
                     two backends holding one name is an error, not a precedence question",
                    held.join(" and "),
                    self.openspec.as_deref().unwrap_or_default()
                )
            }
            Refusal::OpenspecOnly => format!(
                "{name} is held by the openspec backend ({}), and this build cannot read \
                 openspec features yet",
                self.openspec.as_deref().unwrap_or_default()
            ),
        }
    }

    /// The lookup miss: the feature and both candidate roots, the same way for
    /// both selectors and both entry points.
    pub fn miss(&self) -> String {
        let name = self.name;
        let sp = &self.roots.superpowers;
        let spec = &self.roots.spec;
        let os = &self.roots.openspec;
        match self.query {
            Query::Plan => format!(
                "no plan for feature {name} (superpowers: {sp}/plans/YYYY-MM-DD-{name}.md or \
                 {spec}/features/{name}/plan.md; openspec: {os}/changes/{name}/)"
            ),
            Query::Feature => format!(
                "no feature {name} (superpowers: {sp}/specs/YYYY-MM-DD-{name}-design.md, \
                 {sp}/plans/YYYY-MM-DD-{name}.md or {spec}/features/{name}/; \
                 openspec: {os}/changes/{name}/)"
            ),
        }
    }
}

/// `openspec/changes/<f>` when the project holds that active change.
fn openspec_change(project: &Path, name: &str) -> Option<PathBuf> {
    let dir = project.join(crate::openspec::CHANGES).join(name);
    dir.is_dir().then_some(dir)
}

/// The typed spelling of [`openspec_change`]; `None` without a project dir.
fn openspec_change_of(project: Option<&Path>, name: &str) -> Option<String> {
    let project = project?;
    openspec_change(project, name).map(|p| rel(project, &p))
}

/// `vibe-spec plan --feature <f> [--json]`.
pub fn plan(
    cx: &crate::api::Execution<'_>,
    found: &Result<Root, Miss>,
    feature: &str,
    json_out: bool,
) -> (u8, Vec<u8>) {
    let root = match checked(cx, found, "plan", feature) {
        Ok(root) => root,
        Err(e) => return e,
    };
    let forced = match forced_layer(cx, "plan", &root.project) {
        Ok(layer) => layer,
        Err(e) => return e,
    };
    let fallback = root.dir.join("features").join(feature).join("plan.md");
    let superpowers = match forced {
        Some(FeatureLayer::Fallback) => None,
        _ => newest(&root.project.join(PLANS), feature, ".md"),
    };
    let lookup = Lookup {
        query: Query::Plan,
        name: feature,
        roots: Roots::of(&root.project, root),
        superpowers: superpowers.iter().map(|p| rel(&root.project, p)).collect(),
        fallback: fallback.is_file().then(|| rel(&root.project, &fallback)),
        openspec: openspec_change(&root.project, feature).map(|p| rel(&root.project, &p)),
    };
    if let Some(refusal) = lookup.refusal() {
        diag!(cx, "vibe-spec plan: {}", lookup.refusal_message(refusal));
        return (exit::FINDINGS, Vec::new());
    }
    if forced == Some(FeatureLayer::Superpowers) && superpowers.is_none() {
        let other = fallback
            .is_file()
            .then(|| format!("the fallback plan {} exists", rel(&root.project, &fallback)));
        diag!(
            cx,
            "{}",
            layer_refusal(
                "plan",
                FeatureLayer::Superpowers,
                &format!("plan (docs/superpowers/plans/YYYY-MM-DD-{feature}.md)"),
                other
            )
        );
        return (exit::FINDINGS, Vec::new());
    }
    let (path, rows, layer) = match superpowers {
        Some(p) => {
            let rows = match read_plan(cx, &p) {
                Ok(text) => task_rows(&text),
                Err(e) => return e,
            };
            (p, rows, FeatureLayer::Superpowers)
        }
        None => {
            if !fallback.is_file() {
                match forced {
                    Some(forced) => {
                        let other = match forced {
                            FeatureLayer::Fallback => {
                                newest(&root.project.join(PLANS), feature, ".md").map(|p| {
                                    format!(
                                        "the superpowers plan {} exists",
                                        rel(&root.project, &p)
                                    )
                                })
                            }
                            FeatureLayer::Superpowers => None,
                        };
                        diag!(
                            cx,
                            "{}",
                            layer_refusal("plan", forced, &rel(&root.project, &fallback), other)
                        )
                    }
                    None => diag!(cx, "vibe-spec plan: {}", lookup.miss()),
                }
                return (exit::FINDINGS, Vec::new());
            }
            let rows = match read_plan(cx, &fallback) {
                Ok(text) => {
                    let table = table_rows(&text);
                    if table.is_empty() {
                        heading_rows(&text, feature)
                    } else {
                        table
                    }
                }
                Err(e) => return e,
            };
            (fallback, rows, FeatureLayer::Fallback)
        }
    };
    if rows.is_empty() {
        diag!(
            cx,
            "vibe-spec plan: {}: {}",
            rel(&root.project, &path),
            no_rows(layer, feature)
        );
    }
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

/// Why an existing plan answered with no unit rows: the forms the artifact the
/// query actually read accepts, and only those. A superpowers plan carries
/// `### Task <n>:` headings; the fallback plan carries the unit table or
/// `### <feature>/<n>` headings — offering the other layer's form sends the
/// reader to a shape its own layer rejects (#133).
fn no_rows(layer: FeatureLayer, feature: &str) -> String {
    match layer {
        FeatureLayer::Superpowers => {
            "no unit rows in a superpowers plan (accepted: `### Task <n>:` headings)".to_string()
        }
        FeatureLayer::Fallback => format!(
            "no unit rows in a fallback plan (accepted: a `| Unit | Status |` table or \
             `### {feature}/<n>` headings)"
        ),
    }
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
            count_box(line, done, open);
        }
    }
    close(&mut rows, &mut counts);
    rows
}

/// Rows of a fallback feature plan written as `### <feature>/<n>` headings
/// (the format `spec/reference/plan.md` teaches): one row per unit, status
/// from its checkboxes up to the next `##`/`###` heading.
fn heading_rows(text: &[u8], feature: &str) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let mut counts: Option<(usize, usize)> = None;
    let mut fences = Fences::new();
    for line in lines(text) {
        if fences.skip(line) {
            continue;
        }
        if line.starts_with(b"## ") || line.starts_with(b"### ") {
            close(&mut rows, &mut counts);
            if let Some((id, title)) = unit_heading(line, feature) {
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
            count_box(line, done, open);
        }
    }
    close(&mut rows, &mut counts);
    rows
}

/// Counts one `- [x]`/`- [ ]` checkbox line.
fn count_box(line: &[u8], done: &mut usize, open: &mut usize) {
    let t = trim_start_space(line);
    if t.starts_with(b"- [x]") || t.starts_with(b"- [X]") {
        *done += 1;
    } else if t.starts_with(b"- [ ]") {
        *open += 1;
    }
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

/// `### <feature>/<n>[ — <title>]` → (`<feature>/<n>`, title).
fn unit_heading(line: &[u8], feature: &str) -> Option<(String, Option<String>)> {
    let rest = line.strip_prefix(b"### ")?;
    let head = feature.as_bytes();
    if rest.len() <= head.len() || &rest[..head.len()] != head || rest[head.len()] != b'/' {
        return None;
    }
    let number_at = head.len() + 1;
    let number = rest[number_at..]
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .count();
    if number == 0 {
        return None;
    }
    let id = lossy(&rest[..number_at + number]);
    let after = trim_space(&rest[number_at + number..]);
    if after.is_empty() {
        return Some((id, None));
    }
    let title = trim_space(
        ["—", "–", "-", ":"]
            .into_iter()
            .find_map(|sep| after.strip_prefix(sep.as_bytes()))?,
    );
    Some((id, (!title.is_empty()).then(|| lossy(title))))
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
pub fn feature(
    cx: &crate::api::Execution<'_>,
    found: &Result<Root, Miss>,
    name: &str,
    json_out: bool,
) -> (u8, Vec<u8>) {
    let root = match checked(cx, found, "feature", name) {
        Ok(root) => root,
        Err(e) => return e,
    };
    let forced = match forced_layer(cx, "feature", &root.project) {
        Ok(layer) => layer,
        Err(e) => return e,
    };
    let mut docs: Vec<(&'static str, PathBuf)> = Vec::new();
    if forced != Some(FeatureLayer::Fallback) {
        docs.extend(superpowers_docs(&root.project, name));
    }
    let dir = root.dir.join("features").join(name);
    let fallback_docs = FALLBACK_DOCS
        .iter()
        .any(|kind| dir.join(format!("{kind}.md")).is_file());
    let lookup = Lookup {
        query: Query::Feature,
        name,
        roots: Roots::of(&root.project, root),
        superpowers: docs.iter().map(|(_, p)| rel(&root.project, p)).collect(),
        fallback: fallback_docs.then(|| rel(&root.project, &dir)),
        openspec: openspec_change(&root.project, name).map(|p| rel(&root.project, &p)),
    };
    if let Some(refusal) = lookup.refusal() {
        diag!(cx, "vibe-spec feature: {}", lookup.refusal_message(refusal));
        return (exit::FINDINGS, Vec::new());
    }
    if forced == Some(FeatureLayer::Superpowers) && docs.is_empty() {
        let other = dir
            .is_dir()
            .then(|| format!("the fallback folder {} exists", rel(&root.project, &dir)));
        diag!(
            cx,
            "{}",
            layer_refusal(
                "feature",
                FeatureLayer::Superpowers,
                "design/plan under docs/superpowers/",
                other
            )
        );
        return (exit::FINDINGS, Vec::new());
    }
    let layer = if docs.is_empty() {
        if !dir.is_dir() {
            match forced {
                Some(forced) => {
                    let other = match forced {
                        FeatureLayer::Fallback => {
                            let held = superpowers_docs(&root.project, name);
                            (!held.is_empty()).then(|| {
                                let paths = held
                                    .iter()
                                    .map(|(_, p)| rel(&root.project, p))
                                    .collect::<Vec<_>>()
                                    .join(" and ");
                                format!("the superpowers layer holds {paths}")
                            })
                        }
                        FeatureLayer::Superpowers => None,
                    };
                    diag!(
                        cx,
                        "{}",
                        layer_refusal("feature", forced, &rel(&root.project, &dir), other)
                    )
                }
                None => diag!(cx, "vibe-spec feature: {}", lookup.miss()),
            }
            return (exit::FINDINGS, Vec::new());
        }
        for kind in FALLBACK_DOCS {
            let p = dir.join(format!("{kind}.md"));
            if p.is_file() {
                docs.push((kind, p));
            }
        }
        if docs.is_empty()
            && let Some(forced) = forced
        {
            diag!(
                cx,
                "{}",
                layer_refusal("feature", forced, &rel(&root.project, &dir), None)
            );
            return (exit::FINDINGS, Vec::new());
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

#[derive(Clone, Debug)]
pub struct FeatureRequest {
    pub selector: crate::ContextSelector,
    pub feature: String,
}
#[derive(Clone, Debug)]
pub struct PlanRequest {
    pub selector: crate::ContextSelector,
    pub feature: String,
}
pub(crate) fn bound_files(files: &crate::documents::Files, feature: &str) -> Vec<String> {
    files
        .keys()
        .filter(|p| {
            if p.starts_with(&format!("spec/features/{feature}/")) {
                return true;
            }
            let (dir, suffix) = if p.starts_with("superpowers/specs/") {
                ("superpowers/specs/", "-design.md")
            } else if p.starts_with("superpowers/plans/") {
                ("superpowers/plans/", ".md")
            } else {
                return false;
            };
            let name = &p[dir.len()..];
            let want = format!("-{feature}{suffix}");
            name.len() == 10 + want.len() && is_date(&name.as_bytes()[..10]) && name[10..] == want
        })
        .cloned()
        .collect()
}
pub(crate) fn feature_data(
    files: &crate::documents::Files,
    name: &str,
    forced: Option<FeatureLayer>,
    project: Option<&Path>,
) -> Result<Value, crate::SpecError> {
    if !valid_name(name) {
        return Err(crate::SpecError::new(
            "invalid_feature",
            "expected lowercase-kebab feature name",
        ));
    }
    let mut docs = Vec::new();
    if forced != Some(FeatureLayer::Fallback) {
        for (prefix, suffix, kind) in [
            ("superpowers/specs/", "-design.md", "design"),
            ("superpowers/plans/", ".md", "plan"),
        ] {
            if let Some(path) = bound_files(files, name)
                .into_iter()
                .filter(|p| p.starts_with(prefix) && p.ends_with(suffix))
                .max()
            {
                docs.push(json!({"kind":kind,"path":path}));
            }
        }
    }
    let fallback = format!("spec/features/{name}");
    let lookup = Lookup {
        query: Query::Feature,
        name,
        roots: Roots::logical(),
        superpowers: docs
            .iter()
            .filter_map(|d| d["path"].as_str().map(str::to_string))
            .collect(),
        fallback: FALLBACK_DOCS
            .iter()
            .any(|kind| files.contains_key(&format!("{fallback}/{kind}.md")))
            .then(|| fallback.clone()),
        openspec: openspec_change_of(project, name),
    };
    if let Some(refusal) = lookup.refusal() {
        return Err(crate::SpecError::new(
            refusal.code(),
            lookup.refusal_message(refusal),
        ));
    }
    if forced == Some(FeatureLayer::Superpowers) && docs.is_empty() {
        return Err(crate::SpecError::new(
            "layer_empty",
            format!(
                "`.vibe/spec.json` sets `featureLayer` \"superpowers\" and the superpowers layer has no design or plan for {name}, and a selected layer never switches"
            ),
        ));
    }
    let layer = if docs.is_empty() {
        for kind in FALLBACK_DOCS {
            let p = format!("spec/features/{name}/{kind}.md");
            if files.contains_key(&p) {
                docs.push(json!({"kind": kind,"path": p}));
            }
        }
        if docs.is_empty()
            && let Some(forced) = forced
        {
            return Err(crate::SpecError::new(
                "layer_empty",
                format!(
                    "`.vibe/spec.json` sets `featureLayer` \"{}\" and the {} layer has no docs for {name}, and a selected layer never switches",
                    forced.key(),
                    forced.key()
                ),
            ));
        }
        "fallback"
    } else {
        "superpowers"
    };
    if docs.is_empty() {
        return Err(crate::SpecError::new("feature_not_found", lookup.miss()));
    }
    Ok(json!({"v":1,"feature":name,"layer":layer,"docs":docs}))
}
pub(crate) fn plan_data(
    files: &crate::documents::Files,
    name: &str,
    forced: Option<FeatureLayer>,
    project: Option<&Path>,
) -> Result<Value, crate::SpecError> {
    if !valid_name(name) {
        return Err(crate::SpecError::new(
            "invalid_feature",
            "expected lowercase-kebab feature name",
        ));
    }
    let fallback = format!("spec/features/{name}/plan.md");
    let superpowers = match forced {
        Some(FeatureLayer::Fallback) => None,
        _ => bound_files(files, name)
            .into_iter()
            .filter(|p| p.starts_with("superpowers/plans/"))
            .max(),
    };
    let lookup = Lookup {
        query: Query::Plan,
        name,
        roots: Roots::logical(),
        superpowers: superpowers.iter().cloned().collect(),
        fallback: files.contains_key(&fallback).then(|| fallback.clone()),
        openspec: openspec_change_of(project, name),
    };
    if let Some(refusal) = lookup.refusal() {
        return Err(crate::SpecError::new(
            refusal.code(),
            lookup.refusal_message(refusal),
        ));
    }
    if forced == Some(FeatureLayer::Superpowers) && superpowers.is_none() {
        return Err(crate::SpecError::new(
            "layer_empty",
            format!(
                "`.vibe/spec.json` sets `featureLayer` \"superpowers\" and the superpowers layer has no plan for {name} (the fallback layer holds {fallback}), and a selected layer never switches"
            ),
        ));
    }
    let path = match superpowers {
        Some(path) => path,
        None => fallback,
    };
    let text = files.get(&path).ok_or_else(|| {
        if let Some(forced) = forced {
            crate::SpecError::new(
                "layer_empty",
                format!(
                    "`.vibe/spec.json` sets `featureLayer` \"{}\" and the {} layer has no plan for {name}, and a selected layer never switches",
                    forced.key(),
                    forced.key()
                ),
            )
        } else {
            crate::SpecError::new("plan_not_found", lookup.miss())
        }
    })?;
    let rows = if path.starts_with("superpowers/") {
        task_rows(text)
    } else {
        let table = table_rows(text);
        if table.is_empty() {
            heading_rows(text, name)
        } else {
            table
        }
    };
    Ok(
        json!({"v":1,"feature":name,"rows":rows.iter().map(|r|{let mut v=json!({"id":r.id,"status":r.status});if let Some(title)=&r.title{v["title"]=json!(title);}v}).collect::<Vec<_>>(),"source":{"kind":"plan","path":path}}),
    )
}
