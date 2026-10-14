//! Structural checks over `cargo metadata` output (D19, D20).

use serde_json::Value;

/// The only crates the workspace may hold (sorted).
pub const MEMBERS: [&str; 3] = ["vibe-core", "vibe-parity", "vibe-spec"];
/// The shared library; the one member other members may depend on.
pub const CORE: &str = "vibe-core";
/// External crates a member may depend on directly (normal, dev or build).
pub const ALLOWED_EXTERNAL: [&str; 3] = ["clap", "serde", "serde_json"];
/// Minimum supported Rust version, as `(major, minor, patch)`.
pub const MSRV: (u64, u64, u64) = (1, 88, 0);

fn packages(meta: &Value) -> &[Value] {
    meta.get("packages")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn str_field<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("?")
}

/// Checks `cargo metadata --no-deps --format-version 1` output: the member
/// set is exactly [`MEMBERS`], only [`CORE`] may be depended on by another
/// member, core depends on no member, and every external dependency is in
/// [`ALLOWED_EXTERNAL`]. Returns how many members were examined.
pub fn check_graph(meta: &Value) -> Result<usize, Vec<String>> {
    let mut errors = Vec::new();
    let pkgs = packages(meta);
    let mut names: Vec<&str> = pkgs.iter().map(|p| str_field(p, "name")).collect();
    names.sort_unstable();
    if names != MEMBERS {
        errors.push(format!("workspace members are {names:?}, want {MEMBERS:?}"));
    }
    for pkg in pkgs {
        let name = str_field(pkg, "name");
        let deps = pkg
            .get("dependencies")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        for dep in deps {
            let dep_name = str_field(dep, "name");
            let is_path = dep.get("path").is_some_and(Value::is_string);
            if MEMBERS.contains(&dep_name) {
                if dep_name != CORE || name == CORE {
                    errors.push(format!("{name} depends on tool crate {dep_name}"));
                }
            } else if is_path {
                errors.push(format!(
                    "{name} has a path dependency outside the workspace: {dep_name}"
                ));
            } else if !ALLOWED_EXTERNAL.contains(&dep_name) {
                errors.push(format!(
                    "{name} depends on {dep_name}, outside the allowlist {ALLOWED_EXTERNAL:?}"
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(pkgs.len())
    } else {
        Err(errors)
    }
}

fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let mut parts = v.split('.').map(str::parse::<u64>);
    let major = parts.next()?.ok()?;
    let minor = parts.next().unwrap_or(Ok(0)).ok()?;
    let patch = parts.next().unwrap_or(Ok(0)).ok()?;
    Some((major, minor, patch))
}

/// Checks full `cargo metadata --format-version 1` output: every resolved
/// package declares a `rust_version` no newer than [`MSRV`]. A package
/// without one fails, because it cannot be proven to build at MSRV.
/// Returns how many packages were examined.
pub fn check_msrv(meta: &Value) -> Result<usize, Vec<String>> {
    let mut errors = Vec::new();
    let pkgs = packages(meta);
    for pkg in pkgs {
        let name = str_field(pkg, "name");
        match pkg.get("rust_version").and_then(Value::as_str) {
            None => errors.push(format!("{name} declares no rust_version")),
            Some(v) => match parse_version(v) {
                Some(rv) if rv <= MSRV => {}
                Some(_) => errors.push(format!("{name} needs Rust {v}, above MSRV {MSRV:?}")),
                None => errors.push(format!("{name} has an unreadable rust_version {v:?}")),
            },
        }
    }
    if pkgs.is_empty() {
        errors.push("cargo metadata listed no packages".to_string());
    }
    if errors.is_empty() {
        Ok(pkgs.len())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{check_graph, check_msrv};
    use serde_json::{Value, json};

    fn reg(name: &str) -> Value {
        json!({"name": name, "source": "registry+https://github.com/rust-lang/crates.io-index"})
    }

    fn path(name: &str) -> Value {
        json!({"name": name, "source": null, "path": format!("/w/crates/{name}")})
    }

    fn meta(spec_deps: Vec<Value>, core_deps: Vec<Value>) -> Value {
        json!({"packages": [
            {"name": "vibe-core", "dependencies": core_deps},
            {"name": "vibe-spec", "dependencies": spec_deps},
            {"name": "vibe-parity", "dependencies": [path("vibe-core"), reg("serde_json")]},
        ]})
    }

    #[test]
    fn the_intended_graph_passes() {
        let m = meta(vec![path("vibe-core"), reg("clap")], vec![]);
        assert_eq!(check_graph(&m).unwrap(), 3);
    }

    #[test]
    fn a_tool_crate_depending_on_another_fails() {
        let m = meta(vec![path("vibe-parity")], vec![]);
        let errs = check_graph(&m).unwrap_err();
        assert!(
            errs[0].contains("vibe-spec depends on tool crate vibe-parity"),
            "{errs:?}"
        );
    }

    #[test]
    fn core_depending_on_a_tool_crate_fails() {
        let m = meta(vec![], vec![path("vibe-spec")]);
        assert!(check_graph(&m).is_err());
    }

    #[test]
    fn a_banned_external_crate_fails() {
        let m = meta(vec![reg("regex")], vec![]);
        let errs = check_graph(&m).unwrap_err();
        assert!(errs[0].contains("regex"), "{errs:?}");
    }

    #[test]
    fn an_outside_path_dependency_fails() {
        let m = meta(vec![path("vibe-flow")], vec![]);
        assert!(check_graph(&m).is_err());
    }

    #[test]
    fn an_extra_or_missing_member_fails() {
        let mut m = meta(vec![], vec![]);
        m["packages"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name": "vibe-flow", "dependencies": []}));
        assert!(check_graph(&m).is_err());
        assert!(check_graph(&json!({"packages": []})).is_err());
    }

    #[test]
    fn msrv_accepts_older_and_equal_and_rejects_newer_or_missing() {
        let ok = json!({"packages": [
            {"name": "a", "rust_version": "1.56"},
            {"name": "b", "rust_version": "1.88.0"},
        ]});
        assert_eq!(check_msrv(&ok).unwrap(), 2);
        let newer = json!({"packages": [{"name": "c", "rust_version": "1.89"}]});
        assert!(check_msrv(&newer).unwrap_err()[0].contains("above MSRV"));
        let missing = json!({"packages": [{"name": "d", "rust_version": null}]});
        assert!(check_msrv(&missing).unwrap_err()[0].contains("no rust_version"));
        assert!(check_msrv(&json!({"packages": []})).is_err());
    }
}
