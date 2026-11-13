//! Structural checks over `cargo metadata` output (D19, D20).

use serde_json::Value;

/// The only crates the workspace may hold (sorted).
pub const MEMBERS: [&str; 5] = ["vibe", "vibe-core", "vibe-flow", "vibe-parity", "vibe-spec"];
/// The shared library; all members may depend on it.
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
/// set is exactly [`MEMBERS`], member edges target [`CORE`] except the
/// embedded `vibe -> vibe-spec` edge, core depends on no member, and every
/// external dependency is in [`ALLOWED_EXTERNAL`] or the exact spec-only
/// MCP SDK/runtime selection. Returns how many members were examined.
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
        let Some(deps) = pkg.get("dependencies").and_then(Value::as_array) else {
            errors.push(format!(
                "{name}: cargo metadata lists no dependencies array"
            ));
            continue;
        };
        for dep in deps {
            let dep_name = str_field(dep, "name");
            let is_path = dep.get("path").is_some_and(Value::is_string);
            if MEMBERS.contains(&dep_name) {
                if (dep_name != CORE || name == CORE)
                    && !(name == "vibe" && dep_name == "vibe-spec")
                {
                    errors.push(format!("{name} depends on tool crate {dep_name}"));
                }
            } else if is_path {
                errors.push(format!(
                    "{name} has a path dependency outside the workspace: {dep_name}"
                ));
            } else if name == "vibe-spec" && matches!(dep_name, "rmcp" | "tokio") {
                let wanted: &[&str] = if dep_name == "rmcp" {
                    &["macros", "server", "transport-io"]
                } else {
                    &["io-std", "io-util", "macros", "rt-multi-thread", "sync"]
                };
                let mut features: Vec<_> = dep["features"]
                    .as_array()
                    .map_or(Vec::new(), |a| a.iter().filter_map(Value::as_str).collect());
                features.sort_unstable();
                let expected_req = if dep_name == "rmcp" { "=3.9.0" } else { "^1" };
                if features != wanted
                    || dep["req"] != expected_req
                    || dep["rename"].is_string()
                    || (dep_name == "rmcp" && dep["uses_default_features"] != false)
                {
                    errors.push(format!(
                        "{name}: {dep_name} differs from the reviewed MCP selection"
                    ));
                }
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

/// Check declared MSRVs against [`MSRV`]. Five exact reviewed registry versions
/// have no declaration; they are exceptions, not proof of all-target builds.
/// Every other missing, unreadable or raised declaration fails. Executed build
/// evidence remains separate. Return how many packages were examined.
pub fn check_msrv(meta: &Value) -> Result<usize, Vec<String>> {
    let mut errors = Vec::new();
    let pkgs = packages(meta);
    for pkg in pkgs {
        let name = str_field(pkg, "name");
        match pkg.get("rust_version").and_then(Value::as_str) {
            None if missing_declaration_exception(pkg) => {}
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

fn missing_declaration_exception(pkg: &Value) -> bool {
    pkg["source"] == "registry+https://github.com/rust-lang/crates.io-index"
        && matches!(
            (pkg["name"].as_str(), pkg["version"].as_str()),
            (Some("ident_case"), Some("1.0.1"))
                | (Some("wasi"), Some("0.11.1+wasi-snapshot-preview1"))
                | (Some("android_system_properties"), Some("0.1.6"))
                | (Some("core-foundation-sys"), Some("0.8.7"))
                | (Some("iana-time-zone-haiku"), Some("0.1.2"))
        )
}
/// Normalize Cargo's machine-local IDs while preserving exact reviewed edges,
/// sources, versions and capabilities. A package-name pin alone misses features.
pub fn resolved_graph(meta: &Value) -> Value {
    let pkgs = packages(meta);
    let nodes = meta["resolve"]["nodes"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let mut out = Vec::new();
    for node in nodes {
        let Some(pkg) = pkgs.iter().find(|p| p["id"] == node["id"]) else {
            return Value::Null;
        };
        let mut dependencies = Vec::new();
        for dep in node["deps"].as_array().map_or(&[][..], Vec::as_slice) {
            let Some(target) = pkgs.iter().find(|p| p["id"] == dep["pkg"]) else {
                return Value::Null;
            };
            let mut kinds = dep["dep_kinds"].as_array().cloned().unwrap_or_default();
            kinds.sort_by_key(Value::to_string);
            dependencies.push(serde_json::json!({"package":format!("{}@{}",str_field(target,"name"),str_field(target,"version")),"name":dep["name"],"kinds":kinds}));
        }
        dependencies.sort_by_key(|d| (d["package"].to_string(), d["name"].to_string()));
        let mut features = node["features"].as_array().cloned().unwrap_or_default();
        features.sort_by_key(Value::to_string);
        out.push(serde_json::json!({"name":pkg["name"],"version":pkg["version"],"source":pkg["source"],"features":features,"dependencies":dependencies}));
    }
    out.sort_by_key(|p| (p["name"].to_string(), p["version"].to_string()));
    Value::Array(out)
}
pub fn check_resolved_graph(meta: &Value) -> Result<(), String> {
    let pinned: Value = serde_json::from_str(include_str!(
        "../../../contracts/spec-mcp-dependencies.json"
    ))
    .map_err(|e| e.to_string())?;
    if resolved_graph(meta) == pinned {
        Ok(())
    } else {
        Err("resolved package/version/source/feature/edge graph differs from the reviewed MCP closure".into())
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
            {"name": "vibe-flow", "dependencies": [path("vibe-core"), reg("clap"), reg("serde_json")]},
            {"name": "vibe", "dependencies": [path("vibe-core"), reg("clap")]},
        ]})
    }

    #[test]
    fn the_intended_graph_passes() {
        let m = meta(vec![path("vibe-core"), reg("clap")], vec![]);
        assert_eq!(check_graph(&m).unwrap(), 5);
    }

    #[test]
    fn embedded_spec_edge_is_the_only_tool_exception() {
        // spec-projects-mcp/1: permit the embedding edge, reject flow/core tool edges.
        let mut m = meta(vec![path("vibe-core")], vec![]);
        m["packages"][4]["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(path("vibe-spec"));
        assert_eq!(check_graph(&m).unwrap(), 5);
        m["packages"][3]["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(path("vibe-spec"));
        assert!(
            check_graph(&m)
                .unwrap_err()
                .iter()
                .any(|e| e.contains("vibe-flow depends on tool crate vibe-spec"))
        );
        for tool in ["vibe", "vibe-flow", "vibe-parity", "vibe-spec"] {
            assert!(check_graph(&meta(vec![], vec![path(tool)])).is_err());
        }
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
        let m = meta(vec![path("serde")], vec![]);
        let errs = check_graph(&m).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.contains("path dependency outside the workspace")),
            "{errs:?}"
        );
    }

    #[test]
    fn a_package_without_a_dependencies_array_fails() {
        let mut m = meta(vec![], vec![]);
        m["packages"][1]
            .as_object_mut()
            .unwrap()
            .remove("dependencies");
        let errs = check_graph(&m).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.contains("vibe-spec: cargo metadata lists no dependencies array")),
            "{errs:?}"
        );
    }

    #[test]
    fn an_extra_or_missing_member_fails() {
        let mut m = meta(vec![], vec![]);
        m["packages"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name": "vibe-other", "dependencies": []}));
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
