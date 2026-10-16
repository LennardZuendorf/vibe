//! Shared helpers for the vibe-spec integration tests.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The repository root.
pub fn repo() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// A fresh, canonical scratch project holding an empty `.git/` (bounds discovery).
pub fn project(name: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("vibe-spec-test-{name}-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).unwrap();
    plain(fs::canonicalize(dir).unwrap())
}

/// Drops the Windows verbatim prefix that `fs::canonicalize` adds (`\\?\C:\…`):
/// the binary prints the plain `C:\…` form. A no-op elsewhere.
fn plain(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with(r"UNC\") => PathBuf::from(rest),
        _ => path,
    }
}

/// Writes `text` to `root/rel`, creating parents.
pub fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// What a process produced.
#[derive(Debug)]
pub struct Out {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Out {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
}

fn run(mut cmd: Command, cwd: &Path, env: &[(&str, &str)]) -> Out {
    cmd.current_dir(cwd)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", cwd)
        .env("LC_ALL", "C")
        .env("TZ", "UTC");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let o = cmd.output().unwrap();
    Out {
        code: o.status.code().unwrap_or(-1),
        stdout: o.stdout,
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

/// Runs the candidate in `cwd` with a clean environment plus `env`.
pub fn spec(cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> Out {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vibe-spec"));
    cmd.args(args);
    run(cmd, cwd, env)
}

/// Runs the frozen oracle `oracles/bash/spec/scripts/<script>` the same way.
pub fn oracle(cwd: &Path, script: &str, args: &[&str]) -> Out {
    let mut cmd = Command::new("bash");
    cmd.arg(repo().join("oracles/bash/spec/scripts").join(script))
        .args(args);
    run(cmd, cwd, &[])
}

/// Like [`oracle`], with extra environment variables.
pub fn oracle_env(cwd: &Path, script: &str, args: &[&str], env: &[(&str, &str)]) -> Out {
    let mut cmd = Command::new("bash");
    cmd.arg(repo().join("oracles/bash/spec/scripts").join(script))
        .args(args);
    run(cmd, cwd, env)
}

fn git_cmd(cwd: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.args([
        "-c",
        "commit.gpgsign=false",
        "-c",
        "tag.gpgsign=false",
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "user.email=t@t",
        "-c",
        "user.name=t",
        "-c",
        "init.defaultBranch=main",
    ])
    .args(args)
    .current_dir(cwd)
    .env_remove("GIT_DIR")
    .env_remove("GIT_WORK_TREE");
    cmd
}

/// Runs `git args` in `cwd` with author and committer date `date`; panics on failure.
pub fn git(cwd: &Path, date: &str, args: &[&str]) {
    let out = git_cmd(cwd, args)
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `git args` stdout, trimmed; panics on failure.
pub fn git_out(cwd: &Path, args: &[&str]) -> String {
    let out = git_cmd(cwd, args).output().unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The corpus's F17-off strictness (P3b-R2).
pub const LEGACY_STRICTNESS: &str = include_str!("../../../../tests/parity/legacy-strictness.json");

/// Writes the legacy strictness into the project that holds `cwd` (nearest
/// ancestor with `.git`), unless a config is already there.
pub fn legacy(cwd: &Path) {
    let project = cwd
        .ancestors()
        .find(|d| d.join(".git").exists())
        .unwrap_or(cwd);
    let path = project.join(".vibe/spec.json");
    if !path.exists() {
        write(project, ".vibe/spec.json", LEGACY_STRICTNESS);
    }
}

/// Asserts the candidate's `command args` and the oracle's `script args`
/// agree on exit code and stdout bytes when run in `cwd`.
pub fn assert_parity(cwd: &Path, command: &str, script: &str, args: &[&str]) {
    if command == "validate" {
        legacy(cwd);
    }
    let want = oracle(cwd, script, args);
    let mut full = vec![command];
    full.extend_from_slice(args);
    let got = spec(cwd, &full, &[]);
    assert_eq!(
        (got.code, got.text()),
        (want.code, want.text()),
        "`vibe-spec {command} {args:?}` vs `{script}` in {}\ncandidate stderr: {}",
        cwd.display(),
        got.stderr
    );
}

/// Validates `json` against `$defs/<def>` of `contracts/spec-json.schema.json`
/// with the contracts' own validator (`contracts/lib/schema.mjs`).
#[cfg(unix)]
pub fn assert_spec_json(def: &str, json: &str) {
    let lib = repo().join("contracts/lib/schema.mjs");
    let schema = repo().join("contracts/spec-json.schema.json");
    let script = format!(
        "import {{ validate }} from 'file://{}';\n\
         import {{ readFileSync }} from 'node:fs';\n\
         const root = JSON.parse(readFileSync({:?}, 'utf8'));\n\
         const errs = validate(root, {{ $ref: '#/$defs/{def}' }}, JSON.parse(process.argv[1]));\n\
         if (errs.length) {{ console.error(errs.join('\\n')); process.exit(1); }}",
        lib.display(),
        schema.display().to_string()
    );
    let out = Command::new("node")
        .args(["--input-type=module", "-e", &script, json])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{def} does not validate: {}\n{json}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `vibe-spec validate --format json <extra>` in `cwd`, parsed.
pub fn validate_json(cwd: &Path, extra: &[&str], env: &[(&str, &str)]) -> (i32, serde_json::Value) {
    let mut args = vec!["validate", "--format", "json"];
    args.extend_from_slice(extra);
    let out = spec(cwd, &args, env);
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("not JSON ({e}): {}\nstderr: {}", out.text(), out.stderr));
    (out.code, v)
}

/// A clean layered project (D44): hubs, README/AGENTS, every kind folder,
/// a nested topic folder with its own README hub, and a part.
pub fn layered_tree(name: &str) -> PathBuf {
    let p = project(name);
    let hub = |title: &str, body: &str| {
        format!("---\ntype: hub\nupdated: 2026-01-15\n---\n\n# {title}\n\n{body}\n")
    };
    let note = |t: &str, extra: &str, body: &str| {
        format!("---\ntype: {t}\n{extra}updated: 2026-01-15\n---\n\n{body}\n")
    };
    write(
        &p,
        "docs/spec/README.md",
        &hub(
            "Spec",
            "[Product](product.md) · [Tech](tech.md) · [Design](design.md) · [Plan](plan.md) · [How to write](AGENTS.md)",
        ),
    );
    write(
        &p,
        "docs/spec/AGENTS.md",
        "# Writing the spec\n\nOne fact, one place. Link every note from a hub.\n",
    );
    write(
        &p,
        "docs/spec/product.md",
        &hub(
            "Product",
            "- [Billing](areas/billing/README.md)\n- [Checkout](flows/checkout.md)\n- [Invoice](concepts/invoice.md)\n- [Mobile](parts/mobile/product.md)\n- [Web](parts/web/README.md)",
        ),
    );
    write(
        &p,
        "docs/spec/tech.md",
        &hub(
            "Tech",
            "- [Rust](tech/rust.md)\n- [Errors](platform/errors.md)\n- [Use Rust](decisions/d01-use-rust.md)\n- [Pricing](questions/pricing.md)",
        ),
    );
    write(&p, "docs/spec/design.md", &hub("Design", "Plain text."));
    write(
        &p,
        "docs/spec/plan.md",
        &hub(
            "Plan",
            "| Feature | Status |\n|---|---|\n| checkout | NOT STARTED |",
        ),
    );
    write(&p, "docs/spec/lessons.md", "# Lessons\n");
    write(
        &p,
        "docs/spec/areas/billing/README.md",
        &hub("Billing", "- [Invoices](invoices.md)"),
    );
    write(
        &p,
        "docs/spec/areas/billing/invoices.md",
        &note(
            "area",
            "code: src/billing.rs\n",
            "Issued monthly. See [checkout](../../flows/checkout.md).",
        ),
    );
    write(
        &p,
        "docs/spec/flows/checkout.md",
        &note("flow", "", "Cart to payment."),
    );
    write(
        &p,
        "docs/spec/concepts/invoice.md",
        &note("concept", "", "A bill."),
    );
    write(
        &p,
        "docs/spec/platform/errors.md",
        &note("platform", "", "Errors are typed."),
    );
    write(
        &p,
        "docs/spec/tech/rust.md",
        &note("tech", "applies: \"crates/**/*.rs\"\n", "Edition 2024."),
    );
    write(
        &p,
        "docs/spec/decisions/d01-use-rust.md",
        &note("decision", "", "We use Rust."),
    );
    write(
        &p,
        "docs/spec/questions/pricing.md",
        &note("question", "", "Monthly or yearly?"),
    );
    write(
        &p,
        "docs/spec/parts/mobile/product.md",
        &hub("Mobile", "- [Screens](areas/screens.md)"),
    );
    write(
        &p,
        "docs/spec/parts/mobile/areas/screens.md",
        &note("area", "code: [src/billing.rs]\n", "Two screens."),
    );
    write(
        &p,
        "docs/spec/parts/web/README.md",
        &hub("Web", "- [Product](product.md)"),
    );
    write(
        &p,
        "docs/spec/parts/web/product.md",
        &hub("Web product", "Plain text."),
    );
    write(&p, "src/billing.rs", "// billing\n");
    p
}

/// Runs a frozen oracle copied into `<project>/.oracle/scripts/`, with
/// `project` as cwd. check-drift.sh finds its repo from its own path, so a
/// staged copy treats `project` as the repo.
pub fn staged_oracle(project: &Path, script: &str, args: &[&str]) -> Out {
    let dir = project.join(".oracle").join("scripts");
    fs::create_dir_all(&dir).unwrap();
    fs::copy(
        repo().join("oracles/bash/spec/scripts").join(script),
        dir.join(script),
    )
    .unwrap();
    let mut cmd = Command::new("bash");
    cmd.arg(dir.join(script)).args(args);
    run(cmd, project, &[])
}
