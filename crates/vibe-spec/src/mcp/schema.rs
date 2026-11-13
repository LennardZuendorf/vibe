//! Strict transport contracts and typed request conversion.
use super::*;
use serde_json::Map;

pub(super) const NAMES: &[&str] = &[
    "spec_projects",
    "spec_project_show",
    "spec_project_init",
    "spec_project_attach",
    "spec_branch_init",
    "spec_context",
    "spec_documents",
    "spec_read",
    "spec_write",
    "spec_remove",
    "spec_snapshot",
    "spec_init",
    "spec_validate",
    "spec_feature",
    "spec_plan",
    "spec_lessons",
    "spec_list_specs",
    "spec_scan_merges",
    "spec_trace",
    "spec_drift",
    "spec_promote",
    "spec_export",
    "spec_import",
    "spec_reconcile",
    "spec_reconciliation_status",
    "spec_reconciliation_read",
    "spec_reconciliation_resolve",
    "spec_refresh",
];
fn s() -> Value {
    json!({"type":"string"})
}
fn b() -> Value {
    json!({"type":"boolean"})
}
fn strings() -> Value {
    array(s())
}
fn array(item: Value) -> Value {
    json!({"type":"array","items":item})
}
fn nullable(item: Value) -> Value {
    json!({"anyOf":[item,{"type":"null"}]})
}
fn object(fields: &[(&str, Value)], required: &[&str]) -> Value {
    json!({"type":"object","properties":fields.iter().map(|(k,v)|(k.to_string(),v.clone())).collect::<Map<_,_>>(),"required":required,"additionalProperties":false})
}
fn path() -> Value {
    json!({"anyOf":[s(),object(&[("unix_bytes",array(json!({"type":"integer","minimum":0,"maximum":255})))],&["unix_bytes"]),object(&[("windows_wide",array(json!({"type":"integer","minimum":0,"maximum":65535})))],&["windows_wide"])]})
}
fn selector() -> Vec<(&'static str, Value)> {
    vec![("project", s()), ("checkout", path()), ("branch", s())]
}
fn plan() -> Value {
    object(
        &[
            ("v", json!({"const":1})),
            ("project", s()),
            ("source", s()),
            ("target", s()),
            ("source_ref", s()),
            ("target_ref", s()),
            ("base_spec_oid", nullable(s())),
            ("source_spec_oid", s()),
            ("target_spec_oid", s()),
            ("source_code_head", nullable(s())),
            ("target_code_head", nullable(s())),
            ("target_checkout", nullable(path())),
            ("target_revision", s()),
        ],
        &[
            "v",
            "project",
            "source",
            "target",
            "source_ref",
            "target_ref",
            "base_spec_oid",
            "source_spec_oid",
            "target_spec_oid",
            "source_code_head",
            "target_code_head",
            "target_checkout",
            "target_revision",
        ],
    )
}
fn bundle(deletions: bool) -> Value {
    object(
        &[(
            "files",
            array(object(
                &[
                    ("path", s()),
                    ("content", if deletions { nullable(s()) } else { s() }),
                ],
                &["path", "content"],
            )),
        )],
        &["files"],
    )
}
fn input(name: &str) -> Value {
    let mut fields = selector();
    let mut required = vec![];
    match name {
        "spec_projects" => fields.clear(),
        "spec_project_show" | "spec_branch_init" => {
            fields.retain(|(k, _)| *k != "branch");
            if name == "spec_branch_init" {
                fields.extend([
                    ("name", s()),
                    ("from", s()),
                    ("expected_revision", s()),
                    ("expected_project_revision", s()),
                ]);
                required.extend([
                    "name",
                    "from",
                    "expected_revision",
                    "expected_project_revision",
                ]);
            }
        }
        "spec_project_init" => {
            fields.extend([
                ("mode", json!({"enum":["repo-first","global-first"]})),
                ("name", s()),
                ("expected_revision", s()),
            ]);
            required.extend(["mode", "expected_revision"]);
        }
        "spec_project_attach" => {
            fields = vec![
                ("project", s()),
                ("checkout", path()),
                ("main", b()),
                ("expected_revision", s()),
            ];
            required.extend(["project", "expected_revision"]);
        }
        "spec_read" => {
            fields.push(("path", s()));
            required.push("path");
        }
        "spec_write" => {
            fields.extend([("path", s()), ("content", s()), ("expected_revision", s())]);
            required.extend(["path", "content", "expected_revision"]);
        }
        "spec_remove" => {
            fields.extend([("path", s()), ("expected_revision", s())]);
            required.extend(["path", "expected_revision"]);
        }
        "spec_snapshot" | "spec_init" => {
            fields.push(("expected_revision", s()));
            required.push("expected_revision");
        }
        "spec_validate" => fields.extend([
            ("scope", json!({"enum":["spec-only","with-code"]})),
            ("strict", b()),
        ]),
        "spec_feature" | "spec_plan" => {
            fields.push(("feature", s()));
            required.push("feature");
        }
        "spec_lessons" => {
            fields.push((
                "tags",
                json!({"type":"array","minItems":1,"items":{"type":"string","minLength":1}}),
            ));
            required.push("tags");
        }
        "spec_scan_merges" => fields.push(("feature", s())),
        "spec_trace" => fields.push(("since", s())),
        "spec_drift" => fields.extend([
            ("scope", json!({"enum":["spec-only","with-code"]})),
            ("pre_merge", b()),
        ]),
        "spec_promote" => {
            fields.extend([
                ("feature", s()),
                ("expected_revision", s()),
                ("dry_run", b()),
                ("done", b()),
                ("target", s()),
                ("base", s()),
            ]);
            required.extend(["feature", "expected_revision"]);
        }
        "spec_import" => {
            fields.extend([
                ("expected_revision", s()),
                ("bundle", bundle(false)),
                ("feature", s()),
            ]);
            required.extend(["expected_revision", "bundle"]);
        }
        "spec_reconcile" => {
            fields.extend([
                ("source", s()),
                ("target", s()),
                ("plan", plan()),
                ("expected_revision", s()),
                ("dry_run", b()),
            ]);
        }
        "spec_reconciliation_status"
        | "spec_reconciliation_read"
        | "spec_reconciliation_resolve" => {
            fields = vec![("project", s()), ("record_id", s())];
            required.extend(["project", "record_id"]);
            if name == "spec_reconciliation_read" {
                fields.push(("path", s()));
                required.push("path");
            }
            if name == "spec_reconciliation_resolve" {
                fields.extend([
                    ("expected_target_revision", s()),
                    ("expected_resolution_revision", s()),
                    ("bundle", bundle(true)),
                ]);
                required.extend([
                    "expected_target_revision",
                    "expected_resolution_revision",
                    "bundle",
                ]);
            }
        }
        _ => {}
    }
    let mut result = object(&fields, &required);
    if fields.iter().any(|(k, _)| *k == "checkout") && fields.iter().any(|(k, _)| *k == "branch") {
        result["not"] = json!({"required":["checkout","branch"]});
    }
    if name == "spec_reconcile" {
        let mut preview = selector();
        preview.extend([
            ("source", s()),
            ("target", s()),
            ("dry_run", json!({"const":true})),
        ]);
        let apply = object(
            &[
                ("plan", plan()),
                ("expected_revision", s()),
                ("dry_run", json!({"const":false})),
            ],
            &["plan", "expected_revision"],
        );
        result["anyOf"] = json!([object(&preview, &["source", "target", "dry_run"]), apply]);
    }
    for key in [
        "expected_revision",
        "expected_project_revision",
        "expected_target_revision",
        "expected_resolution_revision",
    ] {
        if result["properties"].get(key).is_some() {
            result["properties"][key]["description"] = json!(match key {
                "expected_project_revision" => "Exact project_revision from spec_project_show.",
                "expected_target_revision" =>
                    "Exact target_revision from spec_reconciliation_read.",
                "expected_resolution_revision" =>
                    "Exact resolution_revision from spec_reconciliation_read; distinct from the canonical target token.",
                _ if matches!(
                    name,
                    "spec_project_init" | "spec_project_attach" | "spec_branch_init"
                ) =>
                    "Exact registry revision from spec_projects (absent for the first initialization).",
                _ =>
                    "Exact context revision_token from spec_context/read, or saved plan.target_revision for reconciliation apply. Never a Git OID.",
            });
        }
    }
    result
}
fn number() -> Value {
    json!({"type":"integer","minimum":0})
}
fn version() -> Value {
    json!({"const":1})
}
fn diagnostic() -> Value {
    object(
        &[
            ("code", s()),
            ("message", s()),
            ("path", s()),
            ("file", s()),
            ("line", number()),
            ("source", s()),
            ("result", validation()),
        ],
        &["code"],
    )
}
fn diagnostics() -> Value {
    array(diagnostic())
}
fn plain_diagnostics() -> Value {
    array(object(
        &[("code", s()), ("message", s())],
        &["code", "message"],
    ))
}
fn validation() -> Value {
    object(
        &[
            ("strict", b()),
            (
                "checks",
                array(object(&[("id", s()), ("level", s())], &["id", "level"])),
            ),
            ("errors", number()),
            ("warnings", number()),
            ("examined", number()),
            ("layout", s()),
            (
                "findings",
                array(object(
                    &[
                        ("check", s()),
                        ("severity", s()),
                        ("file", s()),
                        ("message", s()),
                    ],
                    &["check", "severity", "file", "message"],
                )),
            ),
            ("diagnostics", plain_diagnostics()),
        ],
        &[
            "strict",
            "checks",
            "errors",
            "warnings",
            "examined",
            "layout",
            "findings",
            "diagnostics",
        ],
    )
}
fn refresh() -> Value {
    object(
        &[
            ("applied", strings()),
            ("conflict", strings()),
            ("needs_input", strings()),
            ("diagnostics", diagnostics()),
        ],
        &["applied", "conflict", "needs_input", "diagnostics"],
    )
}
fn context() -> Value {
    object(
        &[
            ("project_id", nullable(s())),
            ("mode", json!({"enum":["repo-first","global-first"]})),
            ("checkout", nullable(path())),
            ("code_ref", nullable(s())),
            ("code_head", nullable(s())),
            ("code_available", b()),
            ("spec_root", path()),
            ("artifact_root", path()),
            ("spec_revision", nullable(s())),
            ("revision_token", s()),
            ("refresh", refresh()),
        ],
        &[
            "project_id",
            "mode",
            "checkout",
            "code_ref",
            "code_head",
            "code_available",
            "spec_root",
            "artifact_root",
            "spec_revision",
            "revision_token",
            "refresh",
        ],
    )
}
fn project() -> Value {
    object(
        &[
            ("id", s()),
            ("name", s()),
            ("mode", json!({"enum":["repo-first","global-first"]})),
            ("main_checkout", path()),
            (
                "checkouts",
                array(object(
                    &[("path", path()), ("common_dir", nullable(path()))],
                    &["path", "common_dir"],
                )),
            ),
            ("remote_aliases", strings()),
        ],
        &[
            "id",
            "name",
            "mode",
            "main_checkout",
            "checkouts",
            "remote_aliases",
        ],
    )
}
fn branch() -> Value {
    object(
        &[
            ("id", s()),
            ("code_ref", s()),
            ("spec_ref", s()),
            ("parent_spec_revision", nullable(s())),
            ("last_code_head", nullable(s())),
            ("last_code_ref", nullable(s())),
        ],
        &[
            "id",
            "code_ref",
            "spec_ref",
            "parent_spec_revision",
            "last_code_head",
            "last_code_ref",
        ],
    )
}
fn receipt() -> Value {
    object(
        &[
            ("context", context()),
            ("diagnostics", diagnostics()),
            (
                "lifecycle_diagnostics",
                array(object(
                    &[("code", s()), ("feature", s()), ("message", s())],
                    &["code", "feature", "message"],
                )),
            ),
            ("recorded", json!({"const":true})),
            ("validated", json!({"const":false})),
        ],
        &[
            "context",
            "diagnostics",
            "lifecycle_diagnostics",
            "recorded",
            "validated",
        ],
    )
}
fn promotion() -> Value {
    let checklist = array(object(
        &[("what", s()), ("target", s())],
        &["what", "target"],
    ));
    let merge = object(
        &[
            ("source", s()),
            ("target", s()),
            ("appended", number()),
            ("skipped", number()),
        ],
        &["source", "target", "appended", "skipped"],
    );
    let delta = object(
        &[
            ("source", s()),
            ("target", s()),
            (
                "changes",
                array(object(
                    &[("op", s()), ("name", s()), ("applied", b())],
                    &["op", "name", "applied"],
                )),
            ),
        ],
        &["source", "target", "changes"],
    );
    let mut variants = Vec::new();
    for preview in [true, false] {
        for (kind, shape) in [
            ("baseline", s()),
            ("checklist", checklist.clone()),
            ("merge", merge.clone()),
        ] {
            let mut fields = vec![
                ("v", version()),
                ("feature", s()),
                ("dry_run", json!({"const":preview})),
                ("artifacts", strings()),
                (
                    "deleted",
                    if preview {
                        json!({"const":[]})
                    } else {
                        strings()
                    },
                ),
                (kind, shape),
            ];
            let mut required = vec!["v", "feature", "dry_run", "artifacts", "deleted", kind];
            if kind == "merge" {
                fields.push(("delta", delta.clone()));
            }
            if !preview {
                fields.push(("receipt", receipt()));
                required.push("receipt");
            }
            variants.push(object(&fields, &required));
        }
    }
    json!({"anyOf":variants})
}

fn data(name: &str) -> Value {
    match name {
        "spec_context" => context(),
        "spec_projects" => object(
            &[
                ("v", version()),
                ("revision", s()),
                ("projects", array(project())),
            ],
            &["v", "revision", "projects"],
        ),
        "spec_project_show" | "spec_project_attach" => object(
            &[
                ("v", version()),
                ("revision", s()),
                ("project_revision", s()),
                ("project", project()),
                ("branches", array(branch())),
            ],
            &["v", "revision", "project_revision", "project", "branches"],
        ),
        "spec_project_init" => object(
            &[
                ("v", version()),
                ("revision", s()),
                ("project_revision", s()),
                ("project", project()),
                ("branches", array(branch())),
                ("context", context()),
            ],
            &[
                "v",
                "revision",
                "project_revision",
                "project",
                "branches",
                "context",
            ],
        ),
        "spec_branch_init" => object(
            &[
                ("v", version()),
                ("revision", s()),
                ("project_revision", s()),
                ("branch", branch()),
                ("context", context()),
            ],
            &["v", "revision", "project_revision", "branch", "context"],
        ),
        "spec_read" => object(
            &[
                ("path", s()),
                ("content", s()),
                ("revision", s()),
                ("context", context()),
            ],
            &["path", "content", "revision", "context"],
        ),
        "spec_documents" => object(
            &[
                (
                    "documents",
                    array(object(
                        &[("path", s()), ("bytes", number()), ("revision", s())],
                        &["path", "bytes", "revision"],
                    )),
                ),
                ("context", context()),
            ],
            &["documents", "context"],
        ),
        "spec_export" => object(
            &[
                (
                    "files",
                    array(object(
                        &[("path", s()), ("content", s())],
                        &["path", "content"],
                    )),
                ),
                ("diagnostics", plain_diagnostics()),
            ],
            &["files", "diagnostics"],
        ),
        "spec_reconciliation_read" => object(
            &[
                ("path", s()),
                ("base", nullable(s())),
                ("source", nullable(s())),
                ("target", nullable(s())),
                ("proposed", nullable(s())),
                ("target_revision", s()),
                ("resolution_revision", s()),
            ],
            &[
                "path",
                "base",
                "source",
                "target",
                "proposed",
                "target_revision",
                "resolution_revision",
            ],
        ),
        "spec_reconciliation_status" | "spec_reconciliation_resolve" => record(),
        "spec_reconcile" => json!({"anyOf":[plan(),record()]}),
        "spec_refresh" => {
            let mut r = refresh();
            r["properties"]["context"] = nullable(context());
            if let Some(required) = r["required"].as_array_mut() {
                required.push(json!("context"));
            }
            r
        }
        "spec_validate" => validation(),
        "spec_feature" => object(
            &[
                ("v", version()),
                ("feature", s()),
                ("layer", s()),
                (
                    "docs",
                    array(object(&[("kind", s()), ("path", s())], &["kind", "path"])),
                ),
            ],
            &["v", "feature", "layer", "docs"],
        ),
        "spec_plan" => object(
            &[
                ("v", version()),
                ("feature", s()),
                (
                    "rows",
                    array(object(
                        &[("id", s()), ("status", s()), ("title", s())],
                        &["id", "status"],
                    )),
                ),
                (
                    "source",
                    object(&[("kind", s()), ("path", s())], &["kind", "path"]),
                ),
            ],
            &["v", "feature", "rows", "source"],
        ),
        "spec_lessons" => object(
            &[
                ("v", version()),
                ("tags", strings()),
                (
                    "lessons",
                    array(object(
                        &[
                            ("title", s()),
                            ("rule", s()),
                            ("tags", strings()),
                            ("pattern", s()),
                            ("date", s()),
                            ("pinned", b()),
                        ],
                        &["title", "rule", "tags"],
                    )),
                ),
            ],
            &["v", "tags", "lessons"],
        ),
        "spec_list_specs" => object(
            &[(
                "documents",
                array(object(
                    &[("path", s()), ("area", s()), ("type", s()), ("scope", s())],
                    &["path", "area", "type", "scope"],
                )),
            )],
            &["documents"],
        ),
        "spec_trace" => object(
            &[
                ("v", version()),
                ("units", number()),
                ("cited", number()),
                ("history", s()),
                ("uncited", strings()),
                (
                    "orphans",
                    array(object(
                        &[("unit", s()), ("source", s())],
                        &["unit", "source"],
                    )),
                ),
                ("diagnostics", plain_diagnostics()),
            ],
            &[
                "v",
                "units",
                "cited",
                "history",
                "uncited",
                "orphans",
                "diagnostics",
            ],
        ),
        "spec_drift" => object(
            &[
                ("v", version()),
                ("root", s()),
                ("pre_merge", b()),
                ("errors", number()),
                ("warnings", number()),
                (
                    "features",
                    array(object(
                        &[("name", s()), ("status", s())],
                        &["name", "status"],
                    )),
                ),
                (
                    "findings",
                    array(object(
                        &[("severity", s()), ("message", s()), ("file", nullable(s()))],
                        &["severity", "message", "file"],
                    )),
                ),
                ("diagnostics", plain_diagnostics()),
            ],
            &[
                "v",
                "root",
                "pre_merge",
                "errors",
                "warnings",
                "features",
                "findings",
                "diagnostics",
            ],
        ),
        "spec_scan_merges" => object(
            &[
                (
                    "blocks",
                    array(object(
                        &[
                            ("feature", s()),
                            ("file", s()),
                            ("start", number()),
                            ("end", number()),
                            ("preview", s()),
                        ],
                        &["feature", "file", "start", "end", "preview"],
                    )),
                ),
                ("diagnostics", diagnostics()),
            ],
            &["blocks", "diagnostics"],
        ),
        "spec_promote" => promotion(),
        "spec_import" => {
            let mut r = receipt();
            r["properties"]["written"] = strings();
            r["properties"]["import_diagnostics"] = plain_diagnostics();
            if let Some(required) = r["required"].as_array_mut() {
                required.extend([json!("written"), json!("import_diagnostics")]);
            }
            r
        }
        _ => receipt(),
    }
}
fn record() -> Value {
    object(
        &[
            ("v", version()),
            ("id", s()),
            ("plan", plan()),
            (
                "state",
                json!({"enum":["pending","applied","conflict","needs-input"]}),
            ),
            ("result_spec_oid", nullable(s())),
            ("conflicts", strings()),
            (
                "diagnostics",
                json!({"anyOf":[{"type":"null"},validation(),diagnostic()]}),
            ),
            (
                "stages",
                json!({"type":"object","additionalProperties":{"type":"array","items":{"type":["string","null"]},"minItems":3,"maxItems":3}}),
            ),
            (
                "merge_index",
                nullable(json!({"type":"object","additionalProperties":s()})),
            ),
        ],
        &[
            "v",
            "id",
            "plan",
            "state",
            "result_spec_oid",
            "conflicts",
            "diagnostics",
            "stages",
            "merge_index",
        ],
    )
}

fn output(name: &str) -> Value {
    let context = if matches!(
        name,
        "spec_projects"
            | "spec_project_show"
            | "spec_project_attach"
            | "spec_reconcile"
            | "spec_reconciliation_status"
            | "spec_reconciliation_read"
            | "spec_reconciliation_resolve"
    ) {
        json!({"type":"null"})
    } else {
        context()
    };
    object(
        &[("context", context), ("data", data(name))],
        &["context", "data"],
    )
}
pub(super) fn tools() -> Vec<Tool> {
    NAMES.iter().map(|&name|{
    let mut tool=Tool::new(name,format!("Typed local spec operation: {name}. Immutable launch selectors are defaults. A per-call project replaces project and inherited checkout/branch; per-call checkout or branch replaces the other anchor. Attach uses its explicit destination project and inherits launch checkout when omitted. Projects lists the global registry. Project show/branch setup select project/checkout metadata, not a stored branch. Saved reconciliation apply uses only its complete captured plan; launch defaults never retarget it. No current project is stored."),input(name).as_object().cloned().unwrap_or_default());
    tool.output_schema=Some(Arc::new(output(name).as_object().cloned().unwrap_or_default()));
    tool.annotations=Some(ToolAnnotations::new().read_only(matches!(name,"spec_projects"|"spec_project_show")).destructive(!matches!(name,"spec_projects"|"spec_project_show")).idempotent(false).open_world(false));
    tool
}).collect()
}

fn invalid(message: impl Into<String>) -> SpecError {
    error("invalid_arguments", message)
}
/// Validate precisely the JSON Schema subset emitted above; domain constructors
/// then enforce opaque identities, native paths and semantic cross-field rules.
fn validate(schema: &Value, value: &Value) -> bool {
    if let Some(negative) = schema.get("not")
        && validate(negative, value)
    {
        return false;
    }
    if let Some(xs) = schema["anyOf"].as_array()
        && !xs.iter().any(|s| validate(s, value))
    {
        return false;
    }
    if let Some(c) = schema.get("const") {
        return value == c;
    }
    if let Some(xs) = schema["enum"].as_array() {
        return xs.contains(value);
    }
    if let Some(required) = schema["required"].as_array()
        && !value.as_object().is_some_and(|o| {
            required
                .iter()
                .all(|k| k.as_str().is_some_and(|k| o.contains_key(k)))
        })
    {
        return false;
    }
    if let Some(types) = schema["type"].as_array() {
        return types
            .iter()
            .any(|kind| validate(&json!({"type":kind}), value));
    }
    match schema["type"].as_str() {
        Some("object") => value.as_object().is_some_and(|o| {
            schema["required"].as_array().is_none_or(|r| {
                r.iter()
                    .all(|k| k.as_str().is_some_and(|k| o.contains_key(k)))
            }) && o.iter().all(|(k, v)| match schema["properties"].get(k) {
                Some(s) => validate(s, v),
                None => match schema.get("additionalProperties") {
                    Some(Value::Bool(false)) => false,
                    Some(s) if s.is_object() => validate(s, v),
                    _ => true,
                },
            })
        }),
        Some("array") => value.as_array().is_some_and(|a| {
            a.len() >= schema["minItems"].as_u64().unwrap_or(0) as usize
                && schema["maxItems"]
                    .as_u64()
                    .is_none_or(|n| a.len() <= n as usize)
                && a.iter().all(|v| validate(&schema["items"], v))
        }),
        Some("string") => value.as_str().is_some_and(|s| {
            s.chars().count() >= schema["minLength"].as_u64().unwrap_or(0) as usize
        }),
        Some("boolean") => value.is_boolean(),
        Some("null") => value.is_null(),
        Some("integer") => value.as_i64().is_some_and(|v| {
            schema["minimum"].as_i64().is_none_or(|n| v >= n)
                && schema["maximum"].as_i64().is_none_or(|n| v <= n)
        }),
        _ => true,
    }
}
fn text(v: &Value, key: &str) -> Result<String, SpecError> {
    v[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid(format!("missing string {key}")))
}
fn opt(v: &Value, key: &str) -> Option<String> {
    v[key].as_str().map(str::to_owned)
}
fn flag(v: &Value, key: &str) -> bool {
    v[key].as_bool().unwrap_or(false)
}
pub(super) fn request(
    name: &str,
    v: &Value,
    defaults: &ContextSelector,
) -> Result<(Option<SpecRequest>, ContextSelector, bool), SpecError> {
    if !validate(&input(name), v) {
        return Err(invalid("arguments do not match the tool schema"));
    }
    fn sizes(v: &Value) -> Result<(), SpecError> {
        match v {
            Value::String(s) if s.len() > MAX_BYTES => {
                Err(error("size_limit", "document exceeds 8 MiB"))
            }
            Value::Object(o) => o.values().try_for_each(sizes),
            Value::Array(a) => a.iter().try_for_each(sizes),
            _ => Ok(()),
        }
    }
    sizes(v)?;
    let mut sel = defaults.clone();
    if let Some(p) = opt(v, "project") {
        sel.project = Some(p.parse()?);
        if name != "spec_project_attach" {
            sel.checkout = None;
            sel.branch = None;
        }
    }
    if let Some(p) = v.get("checkout") {
        sel.checkout = Some(crate::registry::path(p)?);
        sel.branch = None;
    }
    if let Some(p) = opt(v, "branch") {
        sel.branch = Some(p);
        sel.checkout = None;
    }
    if v.get("checkout").is_some() && v.get("branch").is_some() {
        return Err(invalid("checkout and branch are mutually exclusive"));
    }
    let scope = if opt(v, "scope").as_deref() == Some("with-code") {
        ValidationScope::WithCode
    } else {
        ValidationScope::SpecOnly
    };
    let expected = || text(v, "expected_revision");
    let request = match name {
        "spec_projects" => SpecRequest::Projects,
        "spec_project_show" => SpecRequest::ProjectShow {
            selector: sel.clone(),
        },
        "spec_project_init" => SpecRequest::ProjectInit(InitProjectRequest {
            selector: sel.clone(),
            mode: if text(v, "mode")? == "global-first" {
                StorageMode::GlobalFirst
            } else {
                StorageMode::RepoFirst
            },
            name: opt(v, "name"),
            expected_revision: expected()?,
        }),
        "spec_project_attach" => SpecRequest::ProjectAttach(AttachProjectRequest {
            project: text(v, "project")?.parse()?,
            checkout: sel.checkout.clone(),
            main: flag(v, "main"),
            expected_revision: expected()?,
        }),
        "spec_branch_init" => SpecRequest::BranchInit(InitBranchRequest {
            selector: sel.clone(),
            name: text(v, "name")?,
            from: text(v, "from")?,
            expected_revision: expected()?,
            expected_project_revision: text(v, "expected_project_revision")?,
        }),
        "spec_context" => SpecRequest::Context(sel.clone()),
        "spec_documents" => SpecRequest::Documents(sel.clone()),
        "spec_read" => SpecRequest::ReadDocument {
            selector: sel.clone(),
            path: text(v, "path")?,
        },
        "spec_write" => SpecRequest::WriteDocument(WriteDocumentRequest {
            selector: sel.clone(),
            path: text(v, "path")?,
            content: text(v, "content")?,
            expected_revision: expected()?,
        }),
        "spec_remove" => SpecRequest::RemoveDocument(RemoveDocumentRequest {
            selector: sel.clone(),
            path: text(v, "path")?,
            expected_revision: expected()?,
        }),
        "spec_snapshot" => SpecRequest::Snapshot(SnapshotRequest {
            selector: sel.clone(),
            expected_revision: expected()?,
        }),
        "spec_init" => SpecRequest::Init(InitRequest {
            selector: sel.clone(),
            expected_revision: expected()?,
        }),
        "spec_validate" => SpecRequest::Validate(ValidateRequest {
            selector: sel.clone(),
            scope,
            strict: flag(v, "strict"),
        }),
        "spec_feature" => SpecRequest::Feature(FeatureRequest {
            selector: sel.clone(),
            feature: text(v, "feature")?,
        }),
        "spec_plan" => SpecRequest::Plan(PlanRequest {
            selector: sel.clone(),
            feature: text(v, "feature")?,
        }),
        "spec_lessons" => SpecRequest::Lessons(LessonsRequest {
            selector: sel.clone(),
            tags: v["tags"]
                .as_array()
                .ok_or_else(|| invalid("tags"))?
                .iter()
                .map(|v| v.as_str().unwrap_or_default().to_owned())
                .collect(),
        }),
        "spec_list_specs" => SpecRequest::ListSpecs(sel.clone()),
        "spec_scan_merges" => SpecRequest::ScanMerges {
            selector: sel.clone(),
            feature: opt(v, "feature"),
        },
        "spec_trace" => SpecRequest::Trace(TraceRequest {
            selector: sel.clone(),
            since: opt(v, "since"),
        }),
        "spec_drift" => SpecRequest::Drift(DriftRequest {
            selector: sel.clone(),
            scope,
            pre_merge: flag(v, "pre_merge"),
        }),
        "spec_promote" => SpecRequest::Promote(PromoteRequest {
            selector: sel.clone(),
            feature: text(v, "feature")?,
            expected_revision: expected()?,
            dry_run: flag(v, "dry_run"),
            done: flag(v, "done"),
            target: opt(v, "target"),
            base: opt(v, "base"),
        }),
        "spec_export" => SpecRequest::Export(ExportRequest {
            selector: sel.clone(),
        }),
        "spec_import" => SpecRequest::Import(ImportRequest {
            selector: sel.clone(),
            expected_revision: expected()?,
            feature: opt(v, "feature"),
            bundle: OpenSpecBundle {
                files: v["bundle"]["files"]
                    .as_array()
                    .ok_or_else(|| invalid("bundle"))?
                    .iter()
                    .map(|f| {
                        Ok(BundleFile {
                            path: text(f, "path")?,
                            content: text(f, "content")?,
                        })
                    })
                    .collect::<Result<_, SpecError>>()?,
            },
        }),
        "spec_reconcile" => {
            if flag(v, "dry_run") {
                if ["plan", "expected_revision"]
                    .iter()
                    .any(|k| v.get(k).is_some())
                {
                    return Err(invalid("preview accepts source and target only"));
                }
                SpecRequest::ReconcilePlan(ReconcileRequest {
                    selector: sel.clone(),
                    source: text(v, "source")?,
                    target: text(v, "target")?,
                })
            } else {
                if ["project", "checkout", "branch", "source", "target"]
                    .iter()
                    .any(|k| v.get(k).is_some())
                {
                    return Err(invalid(
                        "apply accepts only the exact saved plan and expected_revision",
                    ));
                }
                SpecRequest::ReconcileApply {
                    plan: ReconcilePlan::from_value(&v["plan"])?,
                    expected_revision: expected()?,
                }
            }
        }
        "spec_reconciliation_status" => SpecRequest::ReconciliationStatus {
            project: text(v, "project")?.parse()?,
            record_id: text(v, "record_id")?.parse()?,
        },
        "spec_reconciliation_read" => SpecRequest::ReconciliationRead {
            project: text(v, "project")?.parse()?,
            record_id: text(v, "record_id")?.parse()?,
            path: text(v, "path")?,
        },
        "spec_reconciliation_resolve" => SpecRequest::ReconciliationResolve(ResolveRequest {
            project: text(v, "project")?.parse()?,
            record_id: text(v, "record_id")?.parse()?,
            expected_target_revision: text(v, "expected_target_revision")?,
            expected_resolution_revision: text(v, "expected_resolution_revision")?,
            files: crate::reconcile::resolution_files(&v["bundle"])?,
        }),
        "spec_refresh" => return Ok((None, sel, false)),
        _ => return Err(invalid("unknown spec tool")),
    };
    Ok((
        Some(request),
        sel,
        matches!(
            name,
            "spec_project_init" | "spec_project_attach" | "spec_branch_init"
        ),
    ))
}

pub(super) fn validate_output(name: &str, value: &Value) -> Result<(), SpecError> {
    let schema = output(name);
    if validate(&schema, value) {
        Ok(())
    } else {
        Err(error(
            "invalid_result",
            format!("{name} returned data outside its output schema"),
        ))
    }
}
