//! `vibe-spec export|import --openspec <dir>` (F18, D34, P3c-R14): OpenSpec's
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
    let mut seen: Vec<String> = Vec::new();
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
        let Some(cap) = rel.strip_suffix(".md").filter(|c| !c.is_empty()) else {
            errs.push(format!("{d}/{rel}: no capability name"));
            continue;
        };
        if seen.iter().any(|c| c == cap) {
            errs.push(format!("{d}/{rel}: capability '{cap}' is already exported"));
            continue;
        }
        seen.push(cap.to_string());
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
    // Directories this call creates, leaf first, ending at the first
    // missing ancestor. Rollback removes exactly these with `remove_dir`.
    let mut made: Vec<&Path> = Vec::new();
    let mut at_dir = Some(out_dir);
    while let Some(d) = at_dir {
        match std::fs::symlink_metadata(d) {
            Err(e) if e.kind() == ErrorKind::NotFound => made.push(d),
            _ => break,
        }
        at_dir = d.parent();
    }
    if !made.is_empty()
        && let Err(e) = std::fs::create_dir_all(out_dir)
    {
        for d in &made {
            let _ = std::fs::remove_dir(d);
        }
        return Err(format!("{shown}: {e}"));
    }
    let top = out_dir.join("openspec");
    let cleanup = |note: String| {
        let mut note = note;
        if let Err(e) = std::fs::remove_dir_all(&top) {
            note.push_str(&format!(" — cleanup failed: {e}"));
        }
        for d in &made {
            if let Err(e) = std::fs::remove_dir(d) {
                note.push_str(&format!(" — cleanup failed for {}: {e}", d.display()));
            }
        }
        note
    };
    if let Err(e) = std::fs::create_dir(&top) {
        let msg = if e.kind() == ErrorKind::AlreadyExists {
            format!("{} already exists", at(shown, "openspec"))
        } else {
            format!("{}: {e}", at(shown, "openspec"))
        };
        for d in &made {
            let _ = std::fs::remove_dir(d);
        }
        return Err(msg);
    }
    for (parts, bytes) in files {
        let path = parts.iter().fold(out_dir.to_path_buf(), |p, c| p.join(c));
        let step = match path.parent() {
            Some(parent) => std::fs::create_dir_all(parent).map_err(|e| e.to_string()),
            None => Ok(()),
        }
        .and_then(|()| vibe_core::fs::write_atomic(&path, bytes).map_err(|e| e.to_string()));
        if let Err(e) = step {
            return Err(cleanup(format!("{}: {e}", at(shown, &parts.join("/")))));
        }
    }
    Ok(())
}

/// `dir` as typed, without trailing `/` (the root stays `/`).
fn shown_dir(dir: &Path) -> String {
    let s = dir.to_string_lossy();
    let t = s.trim_end_matches('/');
    if t.is_empty() && s.starts_with('/') {
        "/".to_string()
    } else {
        t.to_string()
    }
}

/// `<shown>/<rest>` without a doubled `/` after the root.
fn at(shown: &str, rest: &str) -> String {
    if shown.ends_with('/') {
        format!("{shown}{rest}")
    } else {
        format!("{shown}/{rest}")
    }
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
    let shown = shown_dir(dir);
    let out_dir: PathBuf = cwd.join(dir);
    if lexical(&out_dir).starts_with(lexical(&root.dir)) {
        eprintln!("export: ERROR: {shown} is inside the spec root — refusing to export into it");
        return (exit::FINDINGS, Vec::new());
    }
    if std::fs::symlink_metadata(out_dir.join("openspec")).is_ok() {
        eprintln!("export: {} already exists", at(&shown, "openspec"));
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
    let mut out = format!(
        "EXPORTED {} file(s) to {}/\n",
        written.len(),
        at(&shown, "openspec")
    );
    for w in &written {
        out += &format!("  {w}\n");
    }
    (exit::OK, out.into_bytes())
}

/// The carried files of an exported design.md; None without the marker.
/// Names outside `CARRIED`, repeated names and a block whose closing fence
/// is not where the length says refuse.
fn uncarry(design: &[u8]) -> Option<Result<Folder, String>> {
    let at = crate::text::find(design, MARK.as_bytes())?;
    let mut rest = design.get(at + MARK.len()..).unwrap_or_default();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    while let Some(h) = crate::text::find(rest, b"\n## vibe:") {
        let parsed = (|| {
            let after = rest.get(h + 9..)?;
            let eol = after.iter().position(|&c| c == b'\n')?;
            let header = lossy(after.get(..eol)?);
            let (name, len) = header.rsplit_once(' ')?;
            let len: usize = len.parse().ok()?;
            let body = after.get(eol + 1..)?.strip_prefix(b"\n")?;
            let fence_end = body.iter().position(|&c| c == b'\n')?;
            let fence = body.get(..fence_end)?.strip_suffix(b"text")?;
            let start = fence_end + 1;
            let content = body.get(start..start.checked_add(len)?)?;
            let mut next = body.get(start + len..)?;
            if !content.ends_with(b"\n") {
                next = next.strip_prefix(b"\n")?;
            }
            let next = next.strip_prefix(fence)?.strip_prefix(b"\n")?;
            Some((name.to_string(), content.to_vec(), next))
        })();
        let Some((name, content, next)) = parsed else {
            return Some(Err("design.md has a malformed vibe block".to_string()));
        };
        if !CARRIED.contains(&name.as_str()) {
            return Some(Err(format!(
                "design.md carries '{name}', which is not a vibe feature file"
            )));
        }
        if files.iter().any(|(n, _)| *n == name) {
            return Some(Err(format!("design.md carries '{name}' twice")));
        }
        files.push((name, content));
        rest = next;
    }
    Some(Ok(files))
}

/// A block with its trailing blank lines replaced by one newline.
fn normal(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    while v.ends_with(b"\n") {
        v.pop();
    }
    v.push(b'\n');
    v
}

/// Requirement blocks per kind: ADDED (plain sections count as ADDED),
/// MODIFIED, REMOVED. Each block ends in one newline; REMOVED bullets
/// (`- \`### Requirement: X\``) become headers.
fn by_kind(text: &[u8]) -> [Vec<Vec<u8>>; 3] {
    let mut out: [Vec<Vec<u8>>; 3] = Default::default();
    for sec in delta::sections(text) {
        let slot = match sec.kind {
            Kind::Plain | Kind::Added => 0,
            Kind::Modified => 1,
            Kind::Removed => 2,
            Kind::Renamed => continue,
        };
        for b in &sec.blocks {
            out[slot].push(normal(text.get(b.start..b.end).unwrap_or_default()));
        }
        if sec.kind == Kind::Removed {
            let mut fences = Fences::new();
            for line in lines(text.get(sec.header..sec.end).unwrap_or_default()) {
                if fences.skip(line) || line.starts_with(b"###") {
                    continue;
                }
                let t = lossy(line);
                let t = t
                    .trim()
                    .trim_start_matches(['-', '*', '+'])
                    .trim()
                    .trim_matches('`');
                if let Some(name) = delta::requirement_name(t.as_bytes()) {
                    let mut h = b"### Requirement: ".to_vec();
                    h.extend(name);
                    h.push(b'\n');
                    out[slot].push(h);
                }
            }
        }
    }
    out
}

/// The body of `## <title>` (case-insensitive) up to the next heading.
fn section_text(md: &[u8], title: &str) -> Option<String> {
    let mut inside = false;
    let mut body: Vec<String> = Vec::new();
    for line in lines(md) {
        let t = lossy(line);
        if let Some(h) = t.strip_prefix("## ") {
            if inside {
                break;
            }
            inside = h.trim().eq_ignore_ascii_case(title);
            continue;
        }
        if inside && t.starts_with('#') {
            break;
        }
        if inside {
            body.push(t);
        }
    }
    let text = body.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// plan.md units from tasks.md checkbox lines (`- [x] 1.1 Title`).
fn plan_from_tasks(tasks: &[u8], id: &str, today: &str) -> Option<Vec<u8>> {
    let mut units: Vec<(String, bool)> = Vec::new();
    for line in lines(tasks) {
        let t = lossy(line);
        let t = t.trim_start();
        let Some(rest) = ["- [", "* [", "+ ["].iter().find_map(|p| t.strip_prefix(p)) else {
            continue;
        };
        let Some((mark, title)) = rest.split_once(']') else {
            continue;
        };
        let title = title.trim();
        let title = match title.split_once(' ') {
            Some((num, rest))
                if num.starts_with(|c: char| c.is_ascii_digit()) && num.contains('.') =>
            {
                rest.trim()
            }
            _ => title,
        };
        units.push((title.to_string(), mark.trim().eq_ignore_ascii_case("x")));
    }
    if units.is_empty() {
        return None;
    }
    let mut out = format!(
        "---\ntype: feature-plan\nfeature: {id}\nparent: ../../plan.md\nupdated: {today}\n---\n\n# Feature: {id} — Plan\n\n## Requirements Trace\n\n| ID | Requirement | Units |\n|---|---|---|\n\n## Units\n"
    );
    for (i, (title, _)) in units.iter().enumerate() {
        out += &format!("\n### {id}/{} — {title}\n", i + 1);
    }
    out += "\n## Progress\n\n| Unit | Status |\n|---|---|\n";
    for (i, (_, d)) in units.iter().enumerate() {
        out += &format!(
            "| {id}/{} | {} |\n",
            i + 1,
            if *d { "DONE" } else { "NOT STARTED" }
        );
    }
    Some(out.into_bytes())
}

/// Whether `path` is a real directory: Ok(false) when absent, and a
/// symlink or any other kind of entry refuses (never followed).
fn real_dir(path: &Path, show: &str) -> Result<bool, String> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => Err(format!("{show} is a symlink, not followed")),
        Ok(m) if m.is_dir() => Ok(true),
        Ok(_) => Err(format!("{show} is not a directory")),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("{show}: {e}")),
    }
}

/// A regular file's bytes; None when absent. A symlink refuses.
fn read_regular(path: &Path, show: &str) -> Result<Option<Vec<u8>>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => Err(format!("{show} is a symlink, not followed")),
        Ok(m) if !m.is_file() => Err(format!("{show} is not a regular file")),
        Ok(_) => read_opt(path, show),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{show}: {e}")),
    }
}

/// Files of one feature folder: names under `features/<id>/`, and bytes.
type Folder = Vec<(String, Vec<u8>)>;

/// One OpenSpec change as files under `<root>/features/<id>/` (P3c-R15).
/// `id` is a directory name of the source tree: it is validated here before
/// it joins any path.
fn import_change(root: &Root, cdir: &Path, id: &str) -> Result<Folder, String> {
    if !valid_name(id) {
        return Err("not a valid feature name (lowercase-kebab)".to_string());
    }
    let fdir = root.dir.join("features").join(id);
    if !lexical(&fdir).starts_with(lexical(&root.dir)) {
        return Err("resolves outside the spec root".to_string());
    }
    let d = root.display();
    match std::fs::symlink_metadata(&fdir) {
        Ok(_) => return Err(format!("{d}/features/{id}/ already exists")),
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => return Err(format!("{d}/features/{id}/: {e}")),
    }
    let sdir = cdir.join("specs");
    let specs: Vec<String> = if real_dir(&sdir, "specs")? {
        files_under(&sdir)?
            .into_iter()
            .filter(|r| r.ends_with("/spec.md"))
            .collect()
    } else {
        Vec::new()
    };
    if specs.len() > 1 {
        return Err(format!(
            "touches {} capabilities; a vibe feature targets one document",
            specs.len()
        ));
    }
    let cap = specs
        .first()
        .map(|r| r.trim_end_matches("/spec.md").to_string());
    let delta_text = match specs.first() {
        Some(r) => read_regular(&sdir.join(r), &format!("specs/{r}"))?.unwrap_or_default(),
        None => Vec::new(),
    };
    if delta::sections(&delta_text)
        .iter()
        .any(|s| s.kind == Kind::Renamed)
    {
        return Err("RENAMED Requirements is not supported".to_string());
    }
    let kinds = by_kind(&delta_text);
    let design = read_regular(&cdir.join("design.md"), "design.md")?;
    match design.as_deref().and_then(uncarry) {
        Some(Err(e)) => Err(e),
        Some(Ok(carried)) => {
            let product = carried
                .iter()
                .find(|(n, _)| n == "product.md")
                .map(|(_, b)| b)
                .ok_or_else(|| "the carried files have no product.md".to_string())?;
            if by_kind(product) != kinds {
                return Err("requirements were edited in OpenSpec; delete design.md's vibe blocks to import it as a new change".to_string());
            }
            Ok(carried)
        }
        None => foreign(root, id, cap, &kinds, cdir, design),
    }
}

/// A change without vibe blocks: synthesized feature docs (P3c-R15).
fn foreign(
    root: &Root,
    id: &str,
    cap: Option<String>,
    kinds: &[Vec<Vec<u8>>; 3],
    cdir: &Path,
    design: Option<Vec<u8>>,
) -> Result<Folder, String> {
    let today = crate::text::today_utc();
    let parent = cap
        .filter(|c| root.dir.join(format!("{c}.md")).is_file())
        .unwrap_or_else(|| "product".to_string());
    let proposal = read_regular(&cdir.join("proposal.md"), "proposal.md")?.unwrap_or_default();
    let why = section_text(&proposal, "why")
        .unwrap_or_else(|| format!("Imported from OpenSpec change {id}."));
    let mut product = format!(
        "---\ntype: feature-product\nfeature: {id}\nsibling: tech.md\nparent: ../../{parent}.md\nupdated: {today}\n---\n\n# Feature: {id} — Product\n\n{why}\n"
    )
    .into_bytes();
    for (kind, blocks) in [Kind::Added, Kind::Modified, Kind::Removed]
        .iter()
        .zip(kinds)
    {
        if blocks.is_empty() {
            continue;
        }
        product.extend(format!("\n## {} Requirements\n\n", kind.as_str()).into_bytes());
        product.extend(blocks.join(&b'\n'));
    }
    let tech = format!(
        "---\ntype: feature-tech\nfeature: {id}\nsibling: product.md\nparent: ../../tech.md\nupdated: {today}\n---\n\n# Feature: {id} — Tech\n\nImported from OpenSpec change {id}; the design is in design.md.\n"
    );
    let mut files: Folder = vec![
        ("product.md".to_string(), product),
        ("tech.md".to_string(), tech.into_bytes()),
    ];
    if let Some(d) = design {
        files.push(("design.md".to_string(), d));
    }
    let tasks = read_regular(&cdir.join("tasks.md"), "tasks.md")?.unwrap_or_default();
    if let Some(plan) = plan_from_tasks(&tasks, id, &today) {
        files.push(("plan.md".to_string(), plan));
    }
    Ok(files)
}

/// The change directories of `<openspec>/changes` by name, byte order. A
/// symlinked entry is reported, not followed.
fn change_ids(changes: &Path, only: Option<&str>) -> Result<(Vec<String>, Vec<String>), String> {
    let mut ids = Vec::new();
    let mut errs = Vec::new();
    if !real_dir(changes, "openspec/changes")? {
        return Ok((ids, errs));
    }
    let entries = std::fs::read_dir(changes).map_err(|e| format!("openspec/changes: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("openspec/changes: {e}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "archive" || only.is_some_and(|o| o != name) {
            continue;
        }
        let meta = std::fs::symlink_metadata(entry.path()).map_err(|e| format!("{name}: {e}"))?;
        if meta.file_type().is_symlink() {
            errs.push(format!("{name}: is a symlink, not followed"));
        } else if meta.is_dir() {
            ids.push(name);
        }
    }
    ids.sort();
    errs.sort();
    Ok((ids, errs))
}

/// Creates `features/` (when missing) and one new folder per change, and
/// writes every file. Nothing here pre-exists (every folder is made with
/// `create_dir`, so an existing entry refuses), so a failure removes
/// exactly what this call made, files first, then directories.
fn write_folders(features: &Path, shown: &str, plan: &[(String, Folder)]) -> Result<(), String> {
    // (path, is_file), in creation order.
    let mut made: Vec<(PathBuf, bool)> = Vec::new();
    let step = write_step(features, shown, plan, &mut made);
    let Err(mut msg) = step else {
        return Ok(());
    };
    let mut restored = 0;
    for (path, is_file) in made.iter().rev() {
        let gone = if *is_file {
            std::fs::remove_file(path)
        } else {
            std::fs::remove_dir(path)
        };
        match gone {
            Ok(()) => restored += 1,
            Err(e) => msg.push_str(&format!(" — cleanup failed for {}: {e}", path.display())),
        }
    }
    if restored > 0 {
        msg.push_str(&format!(" — removed {restored} created path(s)"));
    }
    Err(msg)
}

fn write_step(
    features: &Path,
    shown: &str,
    plan: &[(String, Folder)],
    made: &mut Vec<(PathBuf, bool)>,
) -> Result<(), String> {
    match std::fs::symlink_metadata(features) {
        Ok(m) if m.file_type().is_symlink() => {
            return Err(format!("{shown}/features is a symlink"));
        }
        Ok(m) if !m.is_dir() => return Err(format!("{shown}/features is not a directory")),
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {
            std::fs::create_dir(features).map_err(|e| format!("{shown}/features: {e}"))?;
            made.push((features.to_path_buf(), false));
        }
        Err(e) => return Err(format!("{shown}/features: {e}")),
    }
    for (id, files) in plan {
        let fdir = features.join(id);
        match std::fs::create_dir(&fdir) {
            Ok(()) => made.push((fdir.clone(), false)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                return Err(format!("{shown}/features/{id}/ already exists"));
            }
            Err(e) => return Err(format!("{shown}/features/{id}/: {e}")),
        }
        for (name, bytes) in files {
            let path = fdir.join(name);
            vibe_core::fs::write_atomic(&path, bytes)
                .map_err(|e| format!("{shown}/features/{id}/{name}: {e}"))?;
            made.push((path, true));
        }
    }
    Ok(())
}

/// `vibe-spec import --openspec <dir> [--feature <name>] [--json]`.
/// Never overwrites: an existing feature folder refuses (P3c-R15).
pub fn import(
    found: &Result<Root, Miss>,
    cwd: &Path,
    dir: &Path,
    only: Option<&str>,
    json_out: bool,
) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let refuse = |e: String| {
        eprintln!("import: {e}");
        (exit::FINDINGS, Vec::new())
    };
    let shown = shown_dir(dir);
    let os = cwd.join(dir).join("openspec");
    match real_dir(&os, &at(&shown, "openspec")) {
        Ok(true) => {}
        Ok(false) => return refuse(format!("no openspec/ under {shown}")),
        Err(e) => return refuse(e),
    }
    let (ids, link_errs) = match change_ids(&os.join("changes"), only) {
        Ok(v) => v,
        Err(e) => return refuse(e),
    };
    if ids.is_empty() && link_errs.is_empty() {
        let which = only.map(|o| format!(" '{o}'")).unwrap_or_default();
        return refuse(format!("no change{which} to import"));
    }
    let mut plan: Vec<(String, Folder)> = Vec::new();
    let mut errs: Vec<String> = link_errs;
    for id in &ids {
        match import_change(root, &os.join("changes").join(id), id) {
            Ok(f) => plan.push((id.clone(), f)),
            Err(e) => errs.push(format!("{id}: {e}")),
        }
    }
    if !errs.is_empty() {
        errs.sort();
        for e in errs {
            eprintln!("import: {e}");
        }
        return (exit::FINDINGS, Vec::new());
    }
    let d = root.display();
    if let Err(e) = write_folders(&root.dir.join("features"), &d, &plan) {
        return refuse(e);
    }
    let written: Vec<String> = plan
        .iter()
        .flat_map(|(id, files)| files.iter().map(move |(n, _)| (id, n)))
        .map(|(id, n)| format!("{d}/features/{id}/{n}"))
        .collect();
    if json_out {
        let v = json!({ "v": 1, "direction": "import", "dir": shown, "written": written });
        return (exit::OK, format!("{v}\n").into_bytes());
    }
    let mut out = format!(
        "IMPORTED {} file(s) from {}/\n",
        written.len(),
        at(&shown, "openspec")
    );
    for w in &written {
        out += &format!("  {w}\n");
    }
    (exit::OK, out.into_bytes())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn the_filesystem_root_stays_a_slash() {
        assert_eq!(shown_dir(Path::new("/")), "/");
        assert_eq!(shown_dir(Path::new("//")), "/");
        assert_eq!(shown_dir(Path::new("out/")), "out");
        assert_eq!(shown_dir(Path::new("/tmp/o/")), "/tmp/o");
        assert_eq!(at("/", "openspec"), "/openspec");
        assert_eq!(at("out", "openspec"), "out/openspec");
    }

    #[test]
    fn a_failed_write_removes_what_it_made() {
        let base = std::env::temp_dir().join(format!("vibe-import-rb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let features = base.join("features");
        let plan = vec![
            (
                "a".to_string(),
                vec![("product.md".to_string(), b"x".to_vec())],
            ),
            (
                "b".to_string(),
                vec![("no/such/dir.md".to_string(), b"y".to_vec())],
            ),
        ];
        let err = write_folders(&features, ".spec", &plan).unwrap_err();
        assert!(
            err.starts_with(".spec/features/b/no/such/dir.md: "),
            "{err}"
        );
        assert!(err.ends_with(" — removed 4 created path(s)"), "{err}");
        assert!(!features.exists());
        // An existing features dir stays, and an existing folder refuses.
        std::fs::create_dir_all(features.join("a")).unwrap();
        let err = write_folders(&features, ".spec", &plan).unwrap_err();
        assert_eq!(err, ".spec/features/a/ already exists");
        assert!(features.join("a").is_dir());
        let _ = std::fs::remove_dir_all(&base);
    }
}
