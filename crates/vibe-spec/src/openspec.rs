//! `vibe-spec export --openspec <dir>` (F18, D34, P3c-R14): OpenSpec's
//! grammar and layout as an interchange format. vibe's own layout never
//! changes, and `openspec/specs/` is never written back into vibe's root
//! documents.
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde_json::json;
use vibe_core::exit;

use crate::delta::{self, Block, Kind};
use crate::layered::lexical;
use crate::query::valid_name;
use crate::root::{Miss, Root};
use crate::text::{Fences, lines, lossy, name_of, sub_dirs, walk_files};

/// Starts the carried vibe files in an exported design.md.
const MARK: &str = "<!-- vibe:export v1 -->";
/// The fallback feature files an export carries byte for byte.
const CARRIED: [&str; 5] = [
    "product.md",
    "tech.md",
    "design.md",
    "plan.md",
    "research.md",
];

/// Files to write: path components under the output dir, and bytes.
type Files = Vec<(Vec<String>, Vec<u8>)>;

fn under(parts: &[&str], tail: &str) -> Vec<String> {
    let mut v: Vec<String> = parts.iter().map(|s| (*s).to_string()).collect();
    v.extend(tail.split('/').map(str::to_string));
    v
}

/// The file's bytes; None when it does not exist. Any other error refuses,
/// so no write ever rests on a partial read.
fn read_opt(path: &Path, show: &str) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{show}: {e}")),
    }
}

/// `walk_files` with the refusal text naming `show`.
fn files_under(dir: &Path) -> Result<Vec<String>, String> {
    walk_files(dir).map_err(|e| e.to_string())
}

/// Blocks joined by one blank line, each ending in a newline.
fn join_blocks(text: &[u8], blocks: &[Block]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            out.push(b'\n');
        }
        out.extend_from_slice(text.get(b.start..b.end).unwrap_or_default());
        if !out.ends_with(b"\n") {
            out.push(b'\n');
        }
    }
    out
}

fn duplicate(blocks: &[Block]) -> Option<Vec<u8>> {
    blocks
        .iter()
        .enumerate()
        .find(|(i, b)| blocks.iter().take(*i).any(|o| o.name == b.name))
        .map(|(_, b)| b.name.clone())
}

/// One carried file: `## vibe:<name> <len>`, a fence longer than any
/// backtick run inside, the bytes, a newline when they lack one, the fence.
fn carry(name: &str, bytes: &[u8]) -> Vec<u8> {
    let longest = bytes
        .split(|&c| c != b'`')
        .map(<[u8]>::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    let mut out = format!("\n## vibe:{name} {}\n\n{fence}text\n", bytes.len()).into_bytes();
    out.extend_from_slice(bytes);
    if !bytes.ends_with(b"\n") {
        out.push(b'\n');
    }
    out.extend_from_slice(fence.as_bytes());
    out.push(b'\n');
    out
}

/// The paragraph after the H1: consecutive non-blank lines that are not
/// headings, comments, rules or `**Label:**` lines, joined by spaces.
fn summary(product: &[u8]) -> Option<String> {
    let mut seen_h1 = false;
    let mut para: Vec<String> = Vec::new();
    for line in lines(product) {
        let t = lossy(line).trim().to_string();
        if !seen_h1 {
            seen_h1 = t.starts_with("# ");
            continue;
        }
        let skip = t.is_empty()
            || t.starts_with('#')
            || t.starts_with("<!--")
            || t.starts_with("**")
            || t == "---";
        if skip {
            if para.is_empty() {
                continue;
            }
            break;
        }
        para.push(t);
    }
    (!para.is_empty()).then(|| para.join(" "))
}

/// Whether the plan's Progress row for `id` says DONE (a whole cell).
fn done(plan: &[u8], id: &str) -> bool {
    lines(plan).into_iter().any(|l| {
        let cells: Vec<String> = lossy(l).split('|').map(|c| c.trim().to_string()).collect();
        cells.get(1).is_some_and(|c| c == id)
            && cells.iter().any(|c| {
                c.chars()
                    .filter(char::is_ascii_alphabetic)
                    .collect::<String>()
                    == "DONE"
            })
    })
}

/// tasks.md for a fallback plan's `### <f>/<n>` units: `- [ ] 1.<n> <title>`,
/// `[x]` when Progress says DONE. None without units.
fn tasks(plan: &[u8], f: &str) -> Option<Vec<u8>> {
    let mut out = format!("# Tasks\n\n## 1. {f}\n\n");
    let mut any = false;
    let mut fences = Fences::new();
    let prefix = format!("### {f}/");
    for line in lines(plan) {
        if fences.skip(line) {
            continue;
        }
        let Some(after) = line.strip_prefix(prefix.as_bytes()) else {
            continue;
        };
        let n: String = lossy(after)
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        if !n.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        let title = lossy(after.get(n.len()..).unwrap_or_default());
        let title = title.trim().trim_start_matches(['—', '-', ':']).trim();
        let title = if title.is_empty() {
            format!("{f}/{n}")
        } else {
            title.to_string()
        };
        let mark = if done(plan, &format!("{f}/{n}")) {
            "x"
        } else {
            " "
        };
        out += &format!("- [{mark}] 1.{n} {title}\n");
        any = true;
    }
    any.then(|| out.into_bytes())
}

/// Spec documents outside `features/` and `archive/` (relative paths).
fn docs(dir: &Path) -> Result<Vec<String>, String> {
    Ok(files_under(dir)?
        .into_iter()
        .filter(|r| r.ends_with(".md") && !r.starts_with("features/") && !r.starts_with("archive/"))
        .collect())
}

/// `path` under `root` as a capability path: `/`-joined, `.md` dropped.
fn capability(root: &Path, path: &Path) -> Option<String> {
    let rel = lexical(path)
        .strip_prefix(lexical(root))
        .ok()?
        .to_path_buf();
    let cap = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    let cap = cap.strip_suffix(".md")?.to_string();
    (!cap.is_empty()).then_some(cap)
}

/// What stops a plan: a refusal list (exit 1 either way).
type Refusal = Vec<String>;

/// One fallback feature folder as an OpenSpec change (P3c-R14).
fn change(root: &Root, fdir: &Path, f: &str) -> Result<Files, Refusal> {
    let show = format!("{}/features/{f}", root.display());
    let one = |e: String| vec![e];
    let Some(product) =
        read_opt(&fdir.join("product.md"), &format!("{show}/product.md")).map_err(one)?
    else {
        eprintln!("export: {show}/: no product.md, not exported");
        return Ok(Vec::new());
    };
    if !valid_name(f) {
        return Err(vec![format!(
            "{show}/: '{f}' is not a valid change id (lowercase-kebab)"
        )]);
    }
    let secs = delta::sections(&product);
    if secs.iter().any(|s| s.kind == Kind::Renamed) {
        return Err(vec![format!(
            "{show}/product.md: RENAMED Requirements is not supported; write REMOVED + ADDED"
        )]);
    }
    let target = delta::target(&root.dir, fdir, &product)
        .map_err(|e| vec![format!("{show}/product.md: {e}")])?;
    let cap = capability(&root.dir, &target).ok_or_else(|| {
        vec![format!(
            "{show}/product.md: target {} is not a .md document under the spec root",
            target.display()
        )]
    })?;
    let main = read_opt(&target, &format!("{show}/product.md: target {cap}"))
        .map_err(one)?
        .unwrap_or_default();
    let main_blocks = delta::requirements(&main);
    let mut errs = Vec::new();
    let mut groups: Vec<Vec<u8>> = Vec::new();
    let mut all: Vec<Block> = Vec::new();
    for kind in [Kind::Added, Kind::Modified, Kind::Removed] {
        let blocks: Vec<Block> = secs
            .iter()
            .filter(|s| s.kind == kind || (kind == Kind::Added && s.kind == Kind::Plain))
            .flat_map(|s| s.blocks.clone())
            .collect();
        if blocks.is_empty() {
            continue;
        }
        if kind == Kind::Modified {
            for b in &blocks {
                let Some(old) = main_blocks.iter().find(|m| m.name == b.name) else {
                    continue;
                };
                let keep = delta::scenarios(product.get(b.start..b.end).unwrap_or_default());
                let lost: Vec<String> =
                    delta::scenarios(main.get(old.start..old.end).unwrap_or_default())
                        .into_iter()
                        .filter(|s| !keep.contains(s))
                        .map(|s| lossy(&s))
                        .collect();
                if !lost.is_empty() {
                    errs.push(format!(
                        "{show}/product.md: MODIFIED '{}' drops scenario(s) the target still has: {}",
                        lossy(&b.name),
                        lost.join(", ")
                    ));
                }
            }
        }
        let mut text = format!("## {} Requirements\n\n", kind.as_str()).into_bytes();
        text.extend(join_blocks(&product, &blocks));
        groups.push(text);
        all.extend(blocks);
    }
    if let Some(dup) = duplicate(&all) {
        errs.push(format!(
            "{show}/product.md: duplicate requirement '{}'",
            lossy(&dup)
        ));
    }
    if !errs.is_empty() {
        return Err(errs);
    }
    let base = ["openspec", "changes", f];
    let mut files: Files = Vec::new();
    if groups.is_empty() {
        files.push((
            under(&base, ".openspec.yaml"),
            b"schema: spec-driven\nskip_specs: true\n".to_vec(),
        ));
    } else {
        files.push((
            under(&base, &format!("specs/{cap}/spec.md")),
            groups.join(&b'\n'),
        ));
    }
    let why =
        summary(&product).unwrap_or_else(|| format!("Change exported by vibe-spec from {show}/."));
    let proposal = format!(
        "# Proposal: {f}\n\n## Why\n{why}\n\n## What Changes\n- Exported from `{show}/` by vibe-spec.\n"
    );
    files.push((under(&base, "proposal.md"), proposal.into_bytes()));
    let plan = read_opt(&fdir.join("plan.md"), &format!("{show}/plan.md")).map_err(one)?;
    if let Some(t) = plan.and_then(|p| tasks(&p, f)) {
        files.push((under(&base, "tasks.md"), t));
    }
    let mut design = format!(
        "# Design: {f}\n\nExported from `{show}/` by vibe-spec. The blocks below carry the vibe files byte for byte; `vibe-spec import --openspec` restores them.\n\n{MARK}\n"
    )
    .into_bytes();
    for name in CARRIED {
        if let Some(bytes) = read_opt(&fdir.join(name), &format!("{show}/{name}")).map_err(one)? {
            design.extend(carry(name, &bytes));
        }
    }
    files.push((under(&base, "design.md"), design));
    for other in files_under(fdir).map_err(one)? {
        if !CARRIED.contains(&other.as_str()) {
            eprintln!("export: {show}/{other}: not exported");
        }
    }
    Ok(files)
}

/// Every capability and change of the tree, in memory (P3c-R14).
fn plan_export(root: &Root) -> Result<Files, Refusal> {
    let d = root.display();
    let mut files: Files = Vec::new();
    let mut errs: Vec<String> = Vec::new();
    for rel in docs(&root.dir).map_err(|e| vec![e])? {
        let Some(text) =
            read_opt(&root.dir.join(&rel), &format!("{d}/{rel}")).map_err(|e| vec![e])?
        else {
            continue;
        };
        let blocks = delta::requirements(&text);
        if blocks.is_empty() {
            continue;
        }
        if let Some(dup) = duplicate(&blocks) {
            errs.push(format!(
                "{d}/{rel}: duplicate requirement '{}'",
                lossy(&dup)
            ));
            continue;
        }
        let cap = rel.trim_end_matches(".md");
        let mut spec = format!(
            "# {cap} Specification\n\n## Purpose\nRequirements of {d}/{rel} in the vibe spec, exported by vibe-spec.\n\n## Requirements\n\n"
        )
        .into_bytes();
        spec.extend(join_blocks(&text, &blocks));
        files.push((
            under(&["openspec", "specs"], &format!("{cap}/spec.md")),
            spec,
        ));
    }
    for fdir in sub_dirs(&root.dir.join("features")) {
        match change(root, &fdir, &name_of(&fdir)) {
            Ok(mut f) => files.append(&mut f),
            Err(mut e) => errs.append(&mut e),
        }
    }
    if errs.is_empty() {
        Ok(files)
    } else {
        Err(errs)
    }
}

/// Creates `<out>/openspec/` and writes `files` into it. `<out>` itself is
/// created only when missing, and is never a symlink. `openspec/` is created
/// with `create_dir`, so an existing entry (even a dangling symlink) is
/// refused atomically, and every directory below it is new and ours.
/// Any failure removes what this call created.
fn write_files(shown: &str, out_dir: &Path, files: &Files) -> Result<(), String> {
    match std::fs::symlink_metadata(out_dir) {
        Ok(m) if m.file_type().is_symlink() => return Err(format!("{shown} is a symlink")),
        Ok(m) if !m.is_dir() => return Err(format!("{shown} is not a directory")),
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => return Err(format!("{shown}: {e}")),
    }
    let made_out = !out_dir.exists();
    if made_out {
        std::fs::create_dir_all(out_dir).map_err(|e| format!("{shown}: {e}"))?;
    }
    let top = out_dir.join("openspec");
    let cleanup = |note: String| {
        let mut note = note;
        if let Err(e) = std::fs::remove_dir_all(&top) {
            note.push_str(&format!(" — cleanup failed: {e}"));
        }
        if made_out {
            let _ = std::fs::remove_dir(out_dir);
        }
        note
    };
    if let Err(e) = std::fs::create_dir(&top) {
        if made_out {
            let _ = std::fs::remove_dir(out_dir);
        }
        return Err(if e.kind() == ErrorKind::AlreadyExists {
            format!("{shown}/openspec already exists")
        } else {
            format!("{shown}/openspec: {e}")
        });
    }
    for (parts, bytes) in files {
        let path = parts.iter().fold(out_dir.to_path_buf(), |p, c| p.join(c));
        let step = match path.parent() {
            Some(parent) => std::fs::create_dir_all(parent).map_err(|e| e.to_string()),
            None => Ok(()),
        }
        .and_then(|()| vibe_core::fs::write_atomic(&path, bytes).map_err(|e| e.to_string()));
        if let Err(e) = step {
            return Err(cleanup(format!("{shown}/{}: {e}", parts.join("/"))));
        }
    }
    Ok(())
}

/// `vibe-spec export --openspec <dir> [--json]`.
pub fn export(found: &Result<Root, Miss>, cwd: &Path, dir: &Path, json_out: bool) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let shown = dir.to_string_lossy().trim_end_matches('/').to_string();
    let out_dir: PathBuf = cwd.join(dir);
    if std::fs::symlink_metadata(out_dir.join("openspec")).is_ok() {
        eprintln!("export: {shown}/openspec already exists");
        return (exit::FINDINGS, Vec::new());
    }
    let files = match plan_export(root) {
        Ok(f) => f,
        Err(errs) => {
            for e in errs {
                eprintln!("export: {e}");
            }
            return (exit::FINDINGS, Vec::new());
        }
    };
    if let Err(e) = write_files(&shown, &out_dir, &files) {
        eprintln!("export: {e}");
        return (exit::FINDINGS, Vec::new());
    }
    let written: Vec<String> = files.iter().map(|(p, _)| p.join("/")).collect();
    if json_out {
        let v = json!({ "v": 1, "direction": "export", "dir": shown, "written": written });
        return (exit::OK, format!("{v}\n").into_bytes());
    }
    let mut out = format!("EXPORTED {} file(s) to {shown}/openspec/\n", written.len());
    for w in &written {
        out += &format!("  {w}\n");
    }
    (exit::OK, out.into_bytes())
}
