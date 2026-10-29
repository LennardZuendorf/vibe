#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The store model: load, validate, hash, init and import.
mod common;
use common::Home;
use std::fs;
use std::path::Path;
use vibe::paths::{Harness, Paths};
use vibe::store::mcp::{Transport, placeholders};
use vibe::store::{self, import, init, skills};
use vibe_core::dirs::Env;

fn paths(home: &Home) -> Paths {
    let vars = [("HOME".to_string(), home.dir.to_string_lossy().into_owned())];
    Paths::from_env(&Env::new(vars, false)).unwrap()
}

fn load_err(home: &Home) -> String {
    store::load(&paths(home)).expect_err("load must refuse")
}

fn set_mcp(home: &Home, json: &str) {
    home.write_json(".config/vibe/mcp.json", json);
}

#[test]
fn load_reads_the_valid_fixture() {
    let home = Home::with_store("load-valid");
    let s = store::load(&paths(&home)).unwrap();
    assert_eq!(s.config.enabled.get(&Harness::Codex), Some(&true));
    assert!(s.config.skill_sources.is_empty());
    assert_eq!(
        s.instructions.as_deref(),
        Some("# Global rules\n\nUse the shared tools.\n")
    );
    let names: Vec<&str> = s.servers.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["local", "remote", "off", "codex-only"]);
    let local = &s.servers[0];
    assert_eq!(local.transport, Transport::Stdio);
    assert_eq!(local.command.as_deref(), Some("npx"));
    assert_eq!(local.args, ["-y", "some-server"]);
    assert_eq!(
        local.env,
        [
            ("LOG_LEVEL".to_string(), "debug".to_string()),
            (
                "API_TOKEN".to_string(),
                "${VIBE_LOCAL_API_TOKEN}".to_string()
            )
        ]
    );
    assert!(local.enabled);
    let remote = &s.servers[1];
    assert_eq!(remote.transport, Transport::Http);
    assert_eq!(remote.url.as_deref(), Some("https://mcp.example.com/mcp"));
    assert_eq!(remote.headers.len(), 1);
    assert!(!s.servers[2].enabled);
    assert_eq!(s.servers[3].harnesses, Some(vec![Harness::Codex]));
    assert!(s.servers[3].applies_to(Harness::Codex));
    assert!(!s.servers[3].applies_to(Harness::Claude));
    assert!(s.servers[0].applies_to(Harness::Claude));
    assert_eq!(s.skills.len(), 1);
    assert_eq!(s.skills[0].name, "a");
    assert!(s.notes.is_empty());
}

#[test]
fn load_refuses_one_bad_field_at_a_time() {
    let cases: [(&str, &str, &str); 5] = [
        (
            "sse",
            r#"{"mcpServers":{"bad":{"type":"sse","command":"x"}}}"#,
            "bad",
        ),
        (
            "args",
            r#"{"mcpServers":{"bad":{"command":"x","args":"no"}}}"#,
            "bad",
        ),
        (
            "env",
            r#"{"mcpServers":{"bad":{"command":"x","env":{"K":1}}}}"#,
            "bad",
        ),
        (
            "enabled",
            r#"{"mcpServers":{"bad":{"command":"x","enabled":"yes"}}}"#,
            "bad",
        ),
        (
            "harness",
            r#"{"mcpServers":{"bad":{"command":"x","harnesses":["nope"]}}}"#,
            "bad",
        ),
    ];
    for (name, json, server) in cases {
        let home = Home::with_store(&format!("load-bad-{name}"));
        set_mcp(&home, json);
        let err = load_err(&home);
        assert!(err.contains(server), "{name}: {err}");
        assert!(err.contains("mcp.json"), "{name}: {err}");
        assert!(!err.contains('\n'), "{name}: {err}");
        assert!(!err.contains("\"sse\""), "{name}: {err}");
    }
}

#[test]
fn load_tolerates_unknown_fields() {
    let home = Home::with_store("load-unknown");
    set_mcp(
        &home,
        r#"{"extra":1,"mcpServers":{"s":{"command":"x","x":{"deep":true},"timeout":30}}}"#,
    );
    let s = store::load(&paths(&home)).unwrap();
    assert_eq!(s.servers.len(), 1);
    assert_eq!(s.servers[0].command.as_deref(), Some("x"));
}

#[test]
fn load_refuses_invalid_json_naming_the_file() {
    let home = Home::with_store("load-badjson");
    home.write(".config/vibe/mcp.json", "{not json");
    assert!(load_err(&home).contains("mcp.json"));
    let home = Home::with_store("load-badjson-store");
    home.write(".config/vibe/store.json", "[1]");
    assert!(load_err(&home).contains("store.json"));
    let home = Home::with_store("load-badtype");
    home.write_json(".config/vibe/mcp.json", r#"{"mcpServers":[]}"#);
    assert!(load_err(&home).contains("mcp.json"));
    let home = Home::with_store("load-badenabled");
    home.write_json(
        ".config/vibe/store.json",
        r#"{"harnesses":{"codex":{"enabled":"y"}}}"#,
    );
    assert!(load_err(&home).contains("store.json"));
}

#[test]
fn load_accepts_missing_files() {
    let home = Home::new("load-missing");
    let s = store::load(&paths(&home)).unwrap();
    assert!(s.servers.is_empty());
    assert!(s.instructions.is_none());
    assert!(s.skills.is_empty());
    assert!(s.notes.is_empty());
    assert!(s.config.enabled.is_empty());
}

#[test]
fn load_skips_empty_or_invalid_instructions_with_one_note() {
    let home = Home::with_store("load-empty-agents");
    home.write(".config/vibe/AGENTS.md", "");
    let s = store::load(&paths(&home)).unwrap();
    assert!(s.instructions.is_none());
    assert_eq!(s.notes.len(), 1, "{:?}", s.notes);
    assert_eq!(s.servers.len(), 4);

    let home = Home::with_store("load-bad-utf8");
    fs::write(home.path(".config/vibe/AGENTS.md"), [0xff, 0xfe, b'x']).unwrap();
    let s = store::load(&paths(&home)).unwrap();
    assert!(s.instructions.is_none());
    assert_eq!(s.notes.len(), 1, "{:?}", s.notes);
}

#[test]
fn config_expands_tilde_in_sources() {
    let home = Home::with_store("config-tilde");
    // `abs` must be absolute for the host: `/abs/dir` is not absolute on Windows, so the
    // fixture would test the relative branch instead of the absolute one.
    let abs = home.path("abs").join("dir");
    home.write_json(
        ".config/vibe/store.json",
        &format!(
            r#"{{"skills":{{"sources":["~/toolkit",{:?}]}},"harnesses":{{"claude":{{"enabled":false}},"zzz":{{"enabled":true}}}}}}"#,
            abs.to_string_lossy()
        ),
    );
    let p = paths(&home);
    let s = store::load(&p).unwrap();
    assert_eq!(s.config.skill_sources[0], p.home.join("toolkit"));
    assert_eq!(s.config.skill_sources[1], abs);
    assert_eq!(s.config.enabled.get(&Harness::Claude), Some(&false));
    assert_eq!(s.config.enabled.len(), 1);
}

#[test]
fn placeholders_extracts_names() {
    assert_eq!(placeholders("Bearer ${A_B}x${c}"), ["A_B"]);
    assert_eq!(placeholders("${_X1}${Y}"), ["_X1", "Y"]);
    assert!(placeholders("${1A} ${} ${A").is_empty());
}

#[cfg(unix)]
fn symlink(target: &Path, at: &Path) {
    std::os::unix::fs::symlink(target, at).unwrap();
}

#[test]
fn skills_first_wins_and_reports_shadowed() {
    let home = Home::new("skills-shadow");
    home.write("one/dup/SKILL.md", "x");
    home.write("one/zeta/SKILL.md", "x");
    home.write("one/alpha/SKILL.md", "x");
    home.write("one/nodoc/README.md", "x");
    home.write("two/dup/SKILL.md", "y");
    home.write("two/other/SKILL.md", "y");
    #[cfg(unix)]
    {
        home.write("real/linked/SKILL.md", "z");
        symlink(&home.path("real/linked"), &home.path("two/viaLink"));
    }
    let (found, shadowed) = skills::discover(&[
        home.path("one"),
        home.path("missing"),
        home.path("two"),
        home.path("one"),
    ]);
    let names: Vec<&str> = found.iter().map(|s| s.name.as_str()).collect();
    #[cfg(unix)]
    assert_eq!(names, ["alpha", "dup", "zeta", "other", "viaLink"]);
    #[cfg(not(unix))]
    assert_eq!(names, ["alpha", "dup", "zeta", "other"]);
    let dup = found.iter().find(|s| s.name == "dup").unwrap();
    assert_eq!(dup.source, home.path("one"));
    assert_eq!(dup.dir, home.path("one/dup"));
    assert_eq!(shadowed.len(), 1);
    assert_eq!(shadowed[0].name, "dup");
    assert_eq!(shadowed[0].kept, home.path("one/dup"));
    assert_eq!(shadowed[0].dropped, home.path("two/dup"));
}

#[test]
fn store_hash_changes_on_each_input() {
    let home = Home::with_store("hash");
    let hash = |home: &Home| {
        let p = paths(home);
        let s = store::load(&p).unwrap();
        store::store_hash(&p, &s)
    };
    let base = hash(&home);
    assert_eq!(base, hash(&home));
    assert_eq!(base.len(), 16);
    let mut seen = vec![base.clone()];

    home.write(".config/vibe/AGENTS.md", "# Other\n");
    seen.push(hash(&home));
    home.write_json(".config/vibe/store.json", r#"{"v":1,"harnesses":{}}"#);
    seen.push(hash(&home));
    set_mcp(&home, r#"{"mcpServers":{}}"#);
    seen.push(hash(&home));
    fs::rename(
        home.path(".config/vibe/skills/a"),
        home.path(".config/vibe/skills/b"),
    )
    .unwrap();
    seen.push(hash(&home));
    home.write("extra/b/SKILL.md", "x");
    home.write_json(
        ".config/vibe/store.json",
        &format!(
            r#"{{"v":1,"harnesses":{{}},"skills":{{"sources":[{}]}}}}"#,
            vibe_core::json::to_compact(&vibe_core::json::Value::str(
                &home.path("extra").to_string_lossy()
            ))
        ),
    );
    fs::remove_dir_all(home.path(".config/vibe/skills/b")).unwrap();
    seen.push(hash(&home));
    for i in 0..seen.len() {
        for j in i + 1..seen.len() {
            assert_ne!(seen[i], seen[j], "hashes {i} and {j} collide");
        }
    }

    let a = Home::with_store("hash-absent");
    fs::remove_file(a.path(".config/vibe/AGENTS.md")).unwrap();
    let absent = hash(&a);
    a.write(".config/vibe/AGENTS.md", "");
    assert_eq!(absent, hash(&a));
    assert_eq!(absent, hash(&a));
}

#[test]
fn init_creates_skeleton_and_is_idempotent() {
    let home = Home::new("init");
    let p = paths(&home);
    let first = init::init(&p, None).unwrap();
    assert!(first.entries.iter().all(|e| e.created), "{first:?}");
    assert_eq!(first.entries.len(), 4);
    let sj = fs::read(p.store_json()).unwrap();
    assert_eq!(
        String::from_utf8(sj.clone()).unwrap(),
        "{\n  \"v\": 1,\n  \"harnesses\": {},\n  \"skills\": {\n    \"sources\": []\n  }\n}\n"
    );
    assert_eq!(
        fs::read_to_string(p.store_agents()).unwrap(),
        "<!-- vibe store: edit this file; every harness reads it -->\n"
    );
    assert_eq!(
        fs::read_to_string(p.store_mcp()).unwrap(),
        "{\n  \"mcpServers\": {}\n}\n"
    );
    assert!(p.store_skills().is_dir());
    assert!(store::load(&p).unwrap().servers.is_empty());

    fs::write(p.store_agents(), "mine\n").unwrap();
    assert!(init::init_files(&p, None).unwrap().is_empty());
    let second = init::init(&p, None).unwrap();
    assert!(second.entries.iter().all(|e| !e.created), "{second:?}");
    assert_eq!(fs::read(p.store_json()).unwrap(), sj);
    assert_eq!(fs::read_to_string(p.store_agents()).unwrap(), "mine\n");
}

#[test]
fn init_files_lists_only_missing_entries() {
    let home = Home::new("init-files");
    let p = paths(&home);
    fs::create_dir_all(&p.store).unwrap();
    fs::write(p.store_mcp(), "{}").unwrap();
    let files = init::init_files(&p, None).unwrap();
    let got: Vec<_> = files.iter().map(|f| f.path.clone()).collect();
    assert_eq!(got, [p.store_json(), p.store_agents(), p.store_skills()]);
    assert!(files[2].bytes.is_none());
    assert!(files[0].bytes.is_some());
}

#[test]
fn init_copies_instructions() {
    let home = Home::new("init-instr");
    home.write("src.md", "# Mine\n\nbody\n");
    let p = paths(&home);
    init::init(&p, Some(&home.path("src.md"))).unwrap();
    assert_eq!(
        fs::read_to_string(p.store_agents()).unwrap(),
        "# Mine\n\nbody\n"
    );
    let home = Home::new("init-instr-missing");
    let err = init::init(&paths(&home), Some(&home.path("nope.md")))
        .err()
        .unwrap();
    assert!(err.contains("nope.md"), "{err}");
}

#[cfg(unix)]
#[test]
fn init_reads_symlinked_store_file_but_refuses_dangling() {
    for file in ["store.json", "mcp.json", "AGENTS.md"] {
        let home = Home::new(&format!("init-link-{file}"));
        let body = if file == "AGENTS.md" {
            "# mine\n"
        } else {
            "{}"
        };
        home.write("elsewhere", body);
        fs::create_dir_all(home.path(".config/vibe")).unwrap();
        symlink(
            &home.path("elsewhere"),
            &home.path(&format!(".config/vibe/{file}")),
        );
        let report = init::init(&paths(&home), None).unwrap();
        let entry = report
            .entries
            .iter()
            .find(|e| e.path.ends_with(file))
            .unwrap();
        assert!(!entry.created, "{file}");
        assert_eq!(home.read("elsewhere"), body);
        assert!(
            fs::symlink_metadata(home.path(&format!(".config/vibe/{file}")))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
    let home = Home::new("init-link-dangling");
    fs::create_dir_all(home.path(".config/vibe")).unwrap();
    symlink(&home.path("gone"), &home.path(".config/vibe/store.json"));
    assert!(init::init(&paths(&home), None).is_err());
    assert!(!home.path("gone").exists());
}

fn agent_stack() -> &'static str {
    r#"{"mcpServers":{
      "exa":{"type":"stdio","command":"exa-mcp","args":["-q"],"env":{"API_KEY":"sk-2","PATH_X":"/bin"},
             "url":"https://x/mcp?exaApiKey=k3&tools=a","headers":{"Authorization":"Bearer sk-1"},
             "enabled":true,"timeout":30}
    }}"#
}

fn stored(home: &Home) -> vibe_core::json::Value {
    vibe_core::json::parse(&home.read(".config/vibe/mcp.json")).unwrap()
}

#[test]
fn import_replaces_secrets() {
    let home = Home::new("import-secrets");
    home.write_json("in.json", agent_stack());
    let p = paths(&home);
    init::init(&p, None).unwrap();
    let report = import::import(&home.path("in.json"), &p, false).unwrap();
    assert_eq!(report.added, ["exa"]);
    assert_eq!(
        report.vars,
        ["VIBE_EXA_AUTHORIZATION", "API_KEY", "VIBE_EXA_EXAAPIKEY"]
    );
    let v = stored(&home);
    let exa = v.path(&["mcpServers", "exa"]).unwrap();
    assert_eq!(
        exa.path(&["headers", "Authorization"]).unwrap().as_str(),
        Some("${VIBE_EXA_AUTHORIZATION}")
    );
    assert_eq!(
        exa.path(&["env", "API_KEY"]).unwrap().as_str(),
        Some("${API_KEY}")
    );
    assert_eq!(exa.path(&["env", "PATH_X"]).unwrap().as_str(), Some("/bin"));
    assert_eq!(
        exa.path(&["url"]).unwrap().as_str(),
        Some("https://x/mcp?exaApiKey=${VIBE_EXA_EXAAPIKEY}&tools=a")
    );
    assert_eq!(
        vibe_core::json::to_compact(exa.path(&["timeout"]).unwrap()),
        "30"
    );
    let text = home.read(".config/vibe/mcp.json");
    assert!(text.contains("\"timeout\": 30"));
    for needle in ["sk-1", "sk-2", "=k3"] {
        assert!(!text.contains(needle), "{needle} in mcp.json");
    }
    assert!(store::load(&p).is_ok());
}

#[test]
fn import_cli_never_prints_secrets() {
    let home = Home::new("import-cli");
    home.write_json("in.json", agent_stack());
    assert_eq!(home.vibe(&["store", "init"], &[]).code, 0);
    let out = home.vibe(&["store", "import-mcp", "in.json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let stdout = out.text();
    assert!(stdout.contains("added exa"), "{stdout}");
    assert!(stdout.contains("export VIBE_EXA_AUTHORIZATION"), "{stdout}");
    assert!(stdout.contains("export API_KEY"), "{stdout}");
    assert!(stdout.contains("export VIBE_EXA_EXAAPIKEY"), "{stdout}");
    let written = home.read(&format!("{}/mcp.json", home.store_rel()));
    for lit in ["sk-1", "sk-2", "k3"] {
        let needle = if lit == "k3" { "=k3" } else { lit };
        assert!(!stdout.contains(needle), "{needle} in stdout");
        assert!(!out.stderr.contains(needle), "{needle} in stderr");
        assert!(!written.contains(needle), "{needle} in mcp.json");
    }
    assert!(!stdout.contains("Bearer") && !out.stderr.contains("Bearer"));
}

#[test]
fn import_keep_literals_warns() {
    let home = Home::new("import-keep");
    home.write_json("in.json", agent_stack());
    let p = paths(&home);
    init::init(&p, None).unwrap();
    let report = import::import(&home.path("in.json"), &p, true).unwrap();
    assert!(report.vars.is_empty());
    assert_eq!(
        report.warnings,
        ["keeping literal values (--keep-literals)"]
    );
    let text = home.read(".config/vibe/mcp.json");
    assert!(text.contains("Bearer sk-1") && text.contains("sk-2") && text.contains("exaApiKey=k3"));
}

#[test]
fn import_keeps_existing_name() {
    let home = Home::new("import-existing");
    home.write_json("in.json", agent_stack());
    let p = paths(&home);
    init::init(&p, None).unwrap();
    set_mcp(&home, r#"{"mcpServers":{"exa":{"command":"mine"}}}"#);
    let report = import::import(&home.path("in.json"), &p, false).unwrap();
    assert!(report.added.is_empty());
    assert_eq!(report.kept_existing, ["exa"]);
    assert!(report.vars.is_empty());
    let v = stored(&home);
    assert_eq!(
        v.path(&["mcpServers", "exa", "command"]).unwrap().as_str(),
        Some("mine")
    );
}

#[test]
fn import_keeps_already_placeholdered_value() {
    let home = Home::new("import-placeholder");
    home.write_json(
        "in.json",
        r#"{"mcpServers":{"s":{"command":"c","env":{"API_KEY":"${MY_KEY}"},"headers":{"X-Auth":"${MY_AUTH}"}}}}"#,
    );
    let p = paths(&home);
    init::init(&p, None).unwrap();
    let report = import::import(&home.path("in.json"), &p, false).unwrap();
    assert!(report.vars.is_empty(), "{:?}", report.vars);
    let v = stored(&home);
    let s = v.path(&["mcpServers", "s"]).unwrap();
    assert_eq!(
        s.path(&["env", "API_KEY"]).unwrap().as_str(),
        Some("${MY_KEY}")
    );
    assert_eq!(
        s.path(&["headers", "X-Auth"]).unwrap().as_str(),
        Some("${MY_AUTH}")
    );
}

#[test]
fn import_accepts_agent_stack_shape() {
    let home = Home::new("import-shape");
    home.write_json("in.json", agent_stack());
    let p = paths(&home);
    init::init(&p, None).unwrap();
    import::import(&home.path("in.json"), &p, false).unwrap();
    let s = store::load(&p).unwrap();
    assert_eq!(s.servers.len(), 1);
    let exa = &s.servers[0];
    assert_eq!(exa.transport, Transport::Stdio);
    assert!(exa.enabled);
    assert_eq!(exa.args, ["-q"]);
    let v = stored(&home);
    assert_eq!(
        vibe_core::json::to_compact(v.path(&["mcpServers", "exa", "timeout"]).unwrap()),
        "30"
    );
}

#[test]
fn import_disambiguates_colliding_variable_names() {
    let home = Home::new("import-collide");
    home.write_json(
        "in.json",
        r#"{"mcpServers":{"a.b":{"command":"c","env":{"api.key":"1"}},"a-b":{"command":"c","env":{"api.key":"2"}},"a_b":{"command":"c","env":{"api.key":"3"}}}}"#,
    );
    let p = paths(&home);
    init::init(&p, None).unwrap();
    let report = import::import(&home.path("in.json"), &p, false).unwrap();
    assert_eq!(
        report.vars,
        [
            "VIBE_A_B_API_KEY",
            "VIBE_A_B_API_KEY_2",
            "VIBE_A_B_API_KEY_3"
        ]
    );
}

#[test]
fn import_names_env_secrets_after_their_key_and_shares_the_name() {
    let home = Home::new("import-same-name");
    home.write_json(
        "in.json",
        r#"{"mcpServers":{"a":{"command":"c","env":{"TOKEN":"1"}},"b":{"command":"c","env":{"TOKEN":"2"}}}}"#,
    );
    let p = paths(&home);
    init::init(&p, None).unwrap();
    let report = import::import(&home.path("in.json"), &p, false).unwrap();
    assert_eq!(report.vars, ["TOKEN"]);
    let v = stored(&home);
    for s in ["a", "b"] {
        assert_eq!(
            v.path(&["mcpServers", s, "env", "TOKEN"]).unwrap().as_str(),
            Some("${TOKEN}")
        );
    }
}

#[test]
fn import_falls_back_to_vibe_names_when_the_key_name_has_another_purpose() {
    let home = Home::new("import-foreign-name");
    home.write_json(
        "in.json",
        r#"{"mcpServers":{
          "a":{"command":"c","env":{"TOKEN":"1"}},
          "b":{"command":"c","args":["--x=${TOKEN}"]},
          "c":{"command":"c","env":{"OTHER":"${TOKEN}","low":"${X}"}},
          "d":{"command":"c","env":{"lowerkey":"2"}}}}"#,
    );
    let p = paths(&home);
    init::init(&p, None).unwrap();
    let report = import::import(&home.path("in.json"), &p, false).unwrap();
    assert_eq!(report.vars, ["VIBE_A_TOKEN", "VIBE_D_LOWERKEY"]);
    let v = stored(&home);
    assert_eq!(
        v.path(&["mcpServers", "a", "env", "TOKEN"])
            .unwrap()
            .as_str(),
        Some("${VIBE_A_TOKEN}")
    );
    assert_eq!(
        v.path(&["mcpServers", "d", "env", "lowerkey"])
            .unwrap()
            .as_str(),
        Some("${VIBE_D_LOWERKEY}")
    );
}

#[test]
fn import_skips_bad_servers_without_echoing_values() {
    let home = Home::new("import-bad");
    let p = paths(&home);
    init::init(&p, None).unwrap();
    home.write("in.json", "{LEAK-LITERAL-9");
    let err = import::import(&home.path("in.json"), &p, false).unwrap_err();
    assert!(!err.contains("LEAK-LITERAL-9"), "{err}");
    home.write_json(
        "in.json",
        r#"{"mcpServers":{
          "t":{"command":"c","type":"LEAK-LITERAL-9"},
          "h":{"command":"c","harnesses":["LEAK-LITERAL-9"]},
          "e":{"command":"c","env":{"API_KEY":5,"X":"LEAK-LITERAL-9"}},
          "good":{"command":"c"}}}"#,
    );
    let report = import::import(&home.path("in.json"), &p, false).unwrap();
    assert_eq!(report.added, ["good"]);
    assert_eq!(report.warnings.len(), 3, "{:?}", report.warnings);
    for (w, name) in report.warnings.iter().zip(["t", "h", "e"]) {
        assert!(w.contains(&format!("\"{name}\"")), "{w}");
        assert!(!w.contains("LEAK-LITERAL-9"), "{w}");
    }
    assert!(report.warnings[0].contains("unsupported type (allowed: stdio, http)"));
    home.write_json("in.json", r#"{"servers":{}}"#);
    assert!(import::import(&home.path("in.json"), &p, false).is_err());
    assert!(
        !home
            .read(".config/vibe/mcp.json")
            .contains("LEAK-LITERAL-9")
    );
}

#[test]
fn load_refusals_do_not_echo_values() {
    let home = Home::with_store("load-no-echo");
    set_mcp(
        &home,
        r#"{"mcpServers":{"bad":{"command":"x","type":"LEAK-LITERAL-9"}}}"#,
    );
    assert!(!load_err(&home).contains("LEAK-LITERAL-9"));
    set_mcp(
        &home,
        r#"{"mcpServers":{"bad":{"command":"x","harnesses":["LEAK-LITERAL-9"]}}}"#,
    );
    assert!(!load_err(&home).contains("LEAK-LITERAL-9"));
}

#[test]
fn import_never_reuses_a_variable_from_an_earlier_run() {
    let home = Home::new("import-two-runs");
    let p = paths(&home);
    init::init(&p, None).unwrap();
    home.write_json(
        "one.json",
        r#"{"mcpServers":{"a.b":{"command":"c","env":{"api.key":"s1"}}}}"#,
    );
    let one = import::import(&home.path("one.json"), &p, false).unwrap();
    assert_eq!(one.vars, ["VIBE_A_B_API_KEY"]);
    home.write_json(
        "two.json",
        r#"{"mcpServers":{"a-b":{"command":"c","env":{"api.key":"s2"}}}}"#,
    );
    let two = import::import(&home.path("two.json"), &p, false).unwrap();
    assert_eq!(two.vars, ["VIBE_A_B_API_KEY_2"]);
    let v = stored(&home);
    assert_eq!(
        v.path(&["mcpServers", "a-b", "env", "api.key"])
            .unwrap()
            .as_str(),
        Some("${VIBE_A_B_API_KEY_2}")
    );
    assert_eq!(
        v.path(&["mcpServers", "a.b", "env", "api.key"])
            .unwrap()
            .as_str(),
        Some("${VIBE_A_B_API_KEY}")
    );
}

#[test]
fn import_keeps_existing_entries_byte_faithful() {
    let home = Home::new("import-order");
    let p = paths(&home);
    init::init(&p, None).unwrap();
    home.write(
        ".config/vibe/mcp.json",
        r#"{"mcpServers":{"zeta":{"command":"z","weight":1.50,"extra":{"b":1,"a":2}},"alpha":{"url":"https://x/mcp","type":"http","note":"n"}},"other":true}"#,
    );
    home.write_json("in.json", r#"{"mcpServers":{"new":{"command":"n"}}}"#);
    import::import(&home.path("in.json"), &p, false).unwrap();
    let v = stored(&home);
    let servers = v.path(&["mcpServers"]).unwrap().as_object().unwrap();
    assert_eq!(servers.keys().collect::<Vec<_>>(), ["zeta", "alpha", "new"]);
    assert_eq!(
        vibe_core::json::to_compact(servers.get("zeta").unwrap()),
        r#"{"command":"z","weight":1.50,"extra":{"b":1,"a":2}}"#
    );
    assert_eq!(
        vibe_core::json::to_compact(servers.get("alpha").unwrap()),
        r#"{"url":"https://x/mcp","type":"http","note":"n"}"#
    );
    assert!(home.read(".config/vibe/mcp.json").contains("1.50"));
    assert_eq!(v.path(&["other"]).and_then(|o| o.as_bool()), Some(true));
}

#[test]
fn import_warns_about_secrets_it_cannot_replace() {
    let home = Home::new("import-leftovers");
    let p = paths(&home);
    init::init(&p, None).unwrap();
    home.write_json(
        "in.json",
        r#"{"mcpServers":{
          "ui":{"url":"https://user:LEAK-1@h/mcp"},
          "nopw":{"url":"https://user@h/mcp"},
          "tok":{"url":"https://ghp_LEAK-6@h/mcp"},
          "pl":{"url":"https://${USER_VAR}@h/mcp"},
          "fr":{"url":"https://h/mcp#token=LEAK-2"},
          "sc":{"url":"https://h/mcp?a=1;token=LEAK-3"},
          "ar":{"command":"c","args":["--api-key","LEAK-4"]},
          "ar2":{"command":"c","args":["--token=LEAK-5"]},
          "ok":{"command":"c","args":["--token","${T}","--verbose"],"url":"https://h/mcp?mode=x"}}}"#,
    );
    let report = import::import(&home.path("in.json"), &p, false).unwrap();
    let warned: Vec<&str> = report.warnings.iter().map(String::as_str).collect();
    assert_eq!(
        warned,
        [
            "ui: possible secret in url userinfo left unchanged \u{2014} edit it to a ${VAR}",
            "nopw: possible secret in url userinfo left unchanged \u{2014} edit it to a ${VAR}",
            "tok: possible secret in url userinfo left unchanged \u{2014} edit it to a ${VAR}",
            "fr: possible secret in url fragment left unchanged \u{2014} edit it to a ${VAR}",
            "sc: possible secret in url query left unchanged \u{2014} edit it to a ${VAR}",
            "ar: possible secret in args left unchanged \u{2014} edit it to a ${VAR}",
            "ar2: possible secret in args left unchanged \u{2014} edit it to a ${VAR}",
        ]
    );
    assert!(!warned.iter().any(|w| w.contains("LEAK")));
}

#[test]
fn import_refuses_when_the_store_is_busy() {
    use vibe_core::lock::{Acquired, Lock, STALE};
    let home = Home::new("import-busy");
    let p = paths(&home);
    init::init(&p, None).unwrap();
    home.write_json("in.json", r#"{"mcpServers":{"n":{"command":"c"}}}"#);
    let held = Lock::acquire(&p.lock_file(), STALE).unwrap();
    assert!(matches!(held, Acquired::Got(_)));
    let err = import::import(&home.path("in.json"), &p, false).unwrap_err();
    assert_eq!(err, "store busy; retry");
    assert!(!home.read(".config/vibe/mcp.json").contains("\"n\""));
    drop(held);
    assert!(import::import(&home.path("in.json"), &p, false).is_ok());
}

#[test]
fn store_hash_describes_the_loaded_bytes() {
    let home = Home::with_store("hash-loaded");
    let p = paths(&home);
    let s = store::load(&p).unwrap();
    let before = store::store_hash(&p, &s);
    home.write(".config/vibe/AGENTS.md", "changed after load\n");
    set_mcp(&home, r#"{"mcpServers":{}}"#);
    assert_eq!(before, store::store_hash(&p, &s));
}

#[test]
fn mcp_server_debug_redacts_env_and_header_values() {
    let home = Home::new("debug-redact");
    set_mcp(
        &home,
        r#"{"mcpServers":{"s":{"command":"c","env":{"API_KEY":"LEAK-LITERAL-9"},"headers":{"Authorization":"Bearer LEAK-LITERAL-8"}}}}"#,
    );
    let s = store::load(&paths(&home)).unwrap();
    let dbg = format!("{:?} {:?}", s.servers[0], s);
    assert!(dbg.contains("API_KEY") && dbg.contains("Authorization") && dbg.contains("<redacted>"));
    assert!(!dbg.contains("LEAK-LITERAL"), "{dbg}");
    for secret in ["LEAK-LITERAL-9", "LEAK-LITERAL-8"] {
        let bytes = format!("{:?}", secret.as_bytes());
        let dump = bytes.trim_matches(|c| c == '[' || c == ']');
        assert!(!dbg.contains(dump), "byte dump of {secret} in {dbg}");
    }
    assert!(dbg.contains("redacted"));
}

#[test]
fn cli_init_twice_and_import_missing() {
    let home = Home::new("cli-init");
    let first = home.vibe(&["store", "init"], &[]);
    assert_eq!(first.code, 0, "{}", first.stderr);
    assert_eq!(
        first
            .text()
            .lines()
            .filter(|l| l.starts_with("created "))
            .count(),
        4
    );
    let second = home.vibe(&["store", "init"], &[]);
    assert_eq!(second.code, 0, "{}", second.stderr);
    let text = second.text();
    assert_eq!(text.lines().count(), 4, "{text}");
    assert!(text.lines().all(|l| l.starts_with("exists ")), "{text}");

    let out = home.vibe(&["store", "import-mcp", "missing.json"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr.lines().count(), 1, "{}", out.stderr);
    assert!(out.stderr.contains("missing.json"));
}

#[test]
fn load_notes_literal_secret_looking_values_once_per_server() {
    let home = Home::with_store("literal-note");
    set_mcp(
        &home,
        r#"{"mcpServers":{
          "lit":{"type":"http","url":"https://h/mcp","headers":{"Authorization":"Bearer LEAK-LIT-1","X-Other":"LEAK-LIT-2"}},
          "env":{"command":"c","env":{"API_TOKEN":"LEAK-LIT-3","LOG_LEVEL":"debug"}},
          "ok":{"command":"c","env":{"API_TOKEN":"${T}","LOG_LEVEL":"debug"},"headers":{"Authorization":"Bearer ${T}"}},
          "off":{"command":"c","enabled":false,"env":{"API_TOKEN":"LEAK-LIT-4"}}}}"#,
    );
    let s = store::load(&paths(&home)).unwrap();
    assert_eq!(
        s.notes,
        [
            "lit: literal secret-looking value projected \u{2014} prefer ${VAR}",
            "env: literal secret-looking value projected \u{2014} prefer ${VAR}",
        ]
    );
    assert!(!s.notes.iter().any(|n| n.contains("LEAK")));
}

#[test]
fn relative_skill_sources_resolve_against_the_store_dir() {
    let home = Home::with_store("relative-sources");
    home.write(
        ".config/vibe/extra/b/SKILL.md",
        "---\nname: b\ndescription: B.\n---\n",
    );
    home.write_json(
        ".config/vibe/store.json",
        r#"{"v":1,"skills":{"sources":["extra","~/abs-src"]}}"#,
    );
    let p = paths(&home);
    let s = store::load(&p).unwrap();
    assert_eq!(
        s.config.skill_sources,
        [p.store.join("extra"), p.home.join("abs-src")]
    );
    let b = s.skills.iter().find(|k| k.name == "b").unwrap();
    assert!(b.dir.is_absolute());
    assert_eq!(b.dir, p.store.join("extra").join("b"));
}

fn golden(kind: &str, class: &str) -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/fixtures/vibe-store")
        .join(kind)
        .join(class);
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| {
            let path = e.unwrap().path();
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&path).unwrap(),
            )
        })
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no goldens in {}", dir.display());
    out
}

#[test]
fn valid_store_and_mcp_goldens_load() {
    for (name, text) in golden("store", "valid") {
        let home = Home::new("golden-store");
        home.write(".config/vibe/store.json", &text);
        store::load(&paths(&home)).unwrap_or_else(|e| panic!("store/valid/{name}: {e}"));
    }
    for (name, text) in golden("mcp", "valid") {
        let home = Home::new("golden-mcp");
        home.write(".config/vibe/mcp.json", &text);
        store::load(&paths(&home)).unwrap_or_else(|e| panic!("mcp/valid/{name}: {e}"));
    }
    for (name, text) in golden("state", "valid") {
        let home = Home::new("golden-state");
        home.write(".local/state/vibe/state.json", &text);
        vibe::state::load(&paths(&home)).unwrap_or_else(|e| panic!("state/valid/{name}: {e}"));
    }
}

#[test]
fn store_with_another_version_is_refused_and_a_missing_version_is_read_as_1() {
    let home = Home::new("golden-v2");
    home.write(".config/vibe/store.json", "{ \"v\": 2 }");
    let err = load_err(&home);
    assert!(err.contains("\"v\""), "{err}");
    for bad in ["\"1\"", "1.5", "null", "0"] {
        home.write(".config/vibe/store.json", &format!("{{ \"v\": {bad} }}"));
        assert!(load_err(&home).contains("\"v\""), "{bad}");
    }
    home.write(".config/vibe/store.json", "{ \"harnesses\": {} }");
    assert!(store::load(&paths(&home)).is_ok());
    let invalid: Vec<String> = golden("store", "invalid")
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert!(invalid.contains(&"v2.json".to_string()));
}
