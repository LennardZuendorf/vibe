#![allow(clippy::unwrap_used, clippy::expect_used)]
use vibe::paths::Harness;
use vibe::store::mcp::{McpServer, Transport};
use vibe::translate::toml::{basic_string, codex_tables, key, scan_mcp_tables};

fn names(text: &str) -> Vec<String> {
    scan_mcp_tables(text).names.into_iter().collect()
}
use vibe::translate::{CodexBody, CodexServer, Translated, claude, codex, factory};
use vibe_core::json;

fn pairs(p: &[(&str, &str)]) -> Vec<(String, String)> {
    p.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn stdio(name: &str, command: &str) -> McpServer {
    McpServer {
        name: name.to_string(),
        transport: Transport::Stdio,
        command: Some(command.to_string()),
        args: vec![],
        env: vec![],
        url: None,
        headers: vec![],
        enabled: true,
        harnesses: None,
    }
}

fn http(name: &str, url: &str) -> McpServer {
    McpServer {
        transport: Transport::Http,
        command: None,
        url: Some(url.to_string()),
        ..stdio(name, "")
    }
}

fn skip<T: std::fmt::Debug>(t: Translated<T>) -> String {
    match t {
        Translated::Skip(r) => r,
        Translated::Ok(v) => panic!("expected skip, got {v:?}"),
    }
}

fn ok<T: std::fmt::Debug>(t: Translated<T>) -> T {
    match t {
        Translated::Ok(v) => v,
        Translated::Skip(r) => panic!("expected ok, got skip {r}"),
    }
}

#[test]
fn toml_basic_string_escapes() {
    assert_eq!(basic_string("a\"b\\c"), r#""a\"b\\c""#);
    assert_eq!(basic_string("a\nb\tc\rd"), r#""a\nb\tc\rd""#);
    assert_eq!(basic_string("\u{1}"), r#""\u0001""#);
    assert_eq!(basic_string("\u{7f}"), r#""\u007F""#);
    assert_eq!(basic_string("\u{8}\u{c}"), r#""\b\f""#);
    assert_eq!(basic_string("é日本"), "\"é日本\"");
    assert_eq!(basic_string(""), "\"\"");
}

#[test]
fn toml_key_bare_or_quoted() {
    assert_eq!(key("exa"), "exa");
    assert_eq!(key("X-Y_1"), "X-Y_1");
    for odd in ["my.server", "a b", "ü", "x]y", ""] {
        assert_eq!(key(odd), basic_string(odd));
    }
}

fn codex_stdio() -> CodexServer {
    CodexServer {
        name: "exa".into(),
        body: CodexBody::Stdio {
            command: "npx".into(),
            args: vec!["-y".into(), "pkg".into()],
            env: pairs(&[("MODE", "x")]),
            env_vars: vec!["API_KEY".into()],
        },
        enabled: true,
    }
}

fn codex_http() -> CodexServer {
    CodexServer {
        name: "remote".into(),
        body: CodexBody::Http {
            url: "https://x/mcp".into(),
            bearer_token_env_var: Some("TOK".into()),
            http_headers: pairs(&[("X Odd", "v")]),
            env_http_headers: pairs(&[("X-Y", "VAR")]),
        },
        enabled: false,
    }
}

#[test]
fn codex_tables_stdio_and_http_exact() {
    let out = codex_tables(&[codex_stdio(), codex_http()]);
    let want = "[mcp_servers.exa]\n\
command = \"npx\"\n\
args = [\"-y\", \"pkg\"]\n\
env = { MODE = \"x\" }\n\
env_vars = [\"API_KEY\"]\n\
\n\
[mcp_servers.remote]\n\
url = \"https://x/mcp\"\n\
bearer_token_env_var = \"TOK\"\n\
http_headers = { \"X Odd\" = \"v\" }\n\
env_http_headers = { X-Y = \"VAR\" }\n\
enabled = false\n";
    assert_eq!(out, want);
    assert_eq!(codex_tables(&[]), "");
}

#[test]
fn codex_translation_rules() {
    let env = |k: &str, v: &str| McpServer {
        env: pairs(&[(k, v)]),
        ..stdio("s", "x")
    };
    let hdr = |h: &str, v: &str| McpServer {
        headers: pairs(&[(h, v)]),
        ..http("s", "https://u")
    };
    let body = |s: &McpServer| ok(codex(s)).body;
    match body(&env("K", "${K}")) {
        CodexBody::Stdio { env, env_vars, .. } => {
            assert!(env.is_empty());
            assert_eq!(env_vars, vec!["K"]);
        }
        b => panic!("{b:?}"),
    }
    match body(&env("K", "lit")) {
        CodexBody::Stdio { env, env_vars, .. } => {
            assert_eq!(env, pairs(&[("K", "lit")]));
            assert!(env_vars.is_empty());
        }
        b => panic!("{b:?}"),
    }
    assert_eq!(
        skip(codex(&env("K", "${OTHER}"))),
        "env K uses another variable"
    );
    assert_eq!(
        skip(codex(&env("K", "a${K}"))),
        "env K uses another variable"
    );
    match body(&hdr("Authorization", "Bearer ${N}")) {
        CodexBody::Http {
            bearer_token_env_var,
            http_headers,
            env_http_headers,
            ..
        } => {
            assert_eq!(bearer_token_env_var.as_deref(), Some("N"));
            assert!(http_headers.is_empty() && env_http_headers.is_empty());
        }
        b => panic!("{b:?}"),
    }
    match body(&hdr("authorization", "Bearer ${N}")) {
        CodexBody::Http {
            bearer_token_env_var,
            ..
        } => {
            assert_eq!(bearer_token_env_var.as_deref(), Some("N"));
        }
        b => panic!("{b:?}"),
    }
    assert_eq!(
        skip(codex(&hdr("Authorization", "bearer ${N}"))),
        "header Authorization mixes text and a variable"
    );
    match body(&hdr("X", "${N}")) {
        CodexBody::Http {
            env_http_headers, ..
        } => {
            assert_eq!(env_http_headers, pairs(&[("X", "N")]));
        }
        b => panic!("{b:?}"),
    }
    match body(&hdr("X", "lit")) {
        CodexBody::Http { http_headers, .. } => assert_eq!(http_headers, pairs(&[("X", "lit")])),
        b => panic!("{b:?}"),
    }
    assert_eq!(
        skip(codex(&hdr("X", "a${N}"))),
        "header X mixes text and a variable"
    );
    let twice = McpServer {
        headers: pairs(&[
            ("Authorization", "Bearer ${A}"),
            ("authorization", "Bearer ${B}"),
        ]),
        ..http("s", "https://u")
    };
    assert!(skip(codex(&twice)).starts_with("header authorization"));

    let cmd = stdio("s", "${X}/bin");
    assert_eq!(skip(codex(&cmd)), "placeholder in command");
    let args = McpServer {
        args: vec!["--k=${X}".into()],
        ..stdio("s", "x")
    };
    assert_eq!(skip(codex(&args)), "placeholder in args");
    assert_eq!(
        skip(codex(&http("s", "https://${H}/mcp"))),
        "placeholder in url"
    );
    let off = McpServer {
        enabled: false,
        ..stdio("s", "x")
    };
    assert!(!ok(codex(&off)).enabled);
}

#[test]
fn skip_reasons_never_carry_values() {
    let s = McpServer {
        env: pairs(&[("K", "secret-${OTHER}")]),
        ..stdio("s", "x")
    };
    assert!(!skip(codex(&s)).contains("secret"));
    let h = McpServer {
        headers: pairs(&[("X", "secret-${N}")]),
        ..http("s", "https://u")
    };
    assert!(!skip(codex(&h)).contains("secret"));
}

#[test]
fn claude_translation() {
    let s = McpServer {
        args: vec!["-y".into(), "${P}".into()],
        env: pairs(&[("K", "${K}")]),
        ..stdio("exa", "npx")
    };
    assert_eq!(
        json::to_compact(&ok(claude(&s))),
        r#"{"type":"stdio","command":"npx","args":["-y","${P}"],"env":{"K":"${K}"}}"#
    );
    let h = McpServer {
        headers: pairs(&[("Authorization", "Bearer ${T}")]),
        ..http("r", "https://u/${X}")
    };
    assert_eq!(
        json::to_compact(&ok(claude(&h))),
        r#"{"type":"http","url":"https://u/${X}","headers":{"Authorization":"Bearer ${T}"}}"#
    );
    assert_eq!(
        json::to_compact(&ok(claude(&stdio("x", "x")))),
        r#"{"type":"stdio","command":"x"}"#
    );
    let off = McpServer {
        enabled: false,
        ..stdio("x", "x")
    };
    assert_eq!(skip(claude(&off)), "disabled");
    assert_eq!(
        skip(claude(&stdio("a.b", "x"))),
        "name not accepted by claude"
    );
    assert_eq!(skip(claude(&stdio("", "x"))), "name not accepted by claude");
    assert_eq!(
        skip(claude(&stdio("a b", "x"))),
        "name not accepted by claude"
    );
}

#[test]
fn factory_translation() {
    let s = McpServer {
        env: pairs(&[("K", "${K}")]),
        args: vec!["a".into()],
        ..stdio("a.b", "x")
    };
    assert_eq!(
        json::to_compact(&ok(factory(&s))),
        r#"{"type":"stdio","command":"x","args":["a"],"env":{"K":"${K}"}}"#
    );
    let h = McpServer {
        headers: pairs(&[("A", "${T}")]),
        enabled: false,
        ..http("r", "https://u")
    };
    assert_eq!(
        json::to_compact(&ok(factory(&h))),
        r#"{"type":"http","url":"https://u","headers":{"A":"${T}"},"disabled":true}"#
    );
    assert_eq!(skip(factory(&stdio("s", "${X}"))), "placeholder in command");
    let a = McpServer {
        args: vec!["${X}".into()],
        ..stdio("s", "x")
    };
    assert_eq!(skip(factory(&a)), "placeholder in args");
    assert_eq!(
        skip(factory(&http("s", "https://${H}"))),
        "placeholder in url"
    );
}

#[test]
fn missing_target_field_skips() {
    let s = McpServer {
        command: None,
        ..stdio("s", "x")
    };
    assert_eq!(skip(claude(&s)), "command missing");
    assert_eq!(skip(codex(&s)), "command missing");
    assert_eq!(skip(factory(&s)), "command missing");
    let h = McpServer {
        url: None,
        ..http("s", "u")
    };
    assert_eq!(skip(factory(&h)), "url missing");
}

#[test]
fn applies_to_filters_by_harnesses_allow_list() {
    let mut s = stdio("s", "x");
    assert!(s.applies_to(Harness::Claude));
    s.harnesses = Some(vec![Harness::Codex]);
    assert!(!s.applies_to(Harness::Claude));
    assert!(s.applies_to(Harness::Codex));
}

#[test]
fn scan_mcp_tables_finds_all_forms() {
    let text = "\
model = \"x\"
mcp_servers.dotted.command = \"x\"
mcp_servers.\"lit key\".command = 'x'
[mcp_servers.plain]
command = \"a\"
[mcp_servers.sub.env]
A = \"1\"
  [mcp_servers.\"q\\\"uote\"]
[ mcp_servers . spaced ]
[[mcp_servers.arr]]
[mcp_servers.'literal']
# [mcp_servers.commented]
[mcp_servers_other.x]
[other.mcp_servers.y]
mcp_servers_other.z = 1
";
    let mut want = [
        "plain", "sub", "q\"uote", "spaced", "arr", "dotted", "lit key", "literal",
    ];
    want.sort_unstable();
    assert_eq!(names(text), want);
    assert!(!scan_mcp_tables(text).root_inline);
}

#[test]
fn scan_mcp_tables_sees_names_under_a_root_header() {
    assert_eq!(
        names("[mcp_servers]\nfoo.command = \"x\"\n[other]\nbar.x = 1\n"),
        ["foo"]
    );
}

#[test]
fn odd_names_round_trip_through_emit_and_scan() {
    for name in [
        "a.b",
        "a b",
        "ü",
        "x]y",
        "q\"uote",
        "back\\slash",
        "tab\tname",
        "",
    ] {
        let srv = CodexServer {
            name: name.into(),
            body: CodexBody::Stdio {
                command: "c".into(),
                args: vec![],
                env: vec![],
                env_vars: vec![],
            },
            enabled: true,
        };
        let text = codex_tables(&[srv]);
        assert_eq!(names(&text), [name], "{text}");
    }
}

#[test]
fn scan_root_assignment_is_root_inline() {
    for text in [
        "mcp_servers = { a = { command = \"x\" } }\n",
        "mcp_servers = {}\n",
        "model = 1\n\nmcp_servers={}\n",
    ] {
        assert!(scan_mcp_tables(text).root_inline, "{text}");
    }
    assert!(!scan_mcp_tables("mcp_servers_x = {}\n[a]\nmcp_servers = 1\n").root_inline);
}

#[test]
fn scan_handles_comments_crlf_bom_and_multiline_strings() {
    assert_eq!(names("[mcp_servers.a] # trailing\n"), ["a"]);
    assert_eq!(
        names("[mcp_servers.a]\r\ncommand = \"x\"\r\n[mcp_servers.b]\r\n"),
        ["a", "b"]
    );
    assert_eq!(names("\u{feff}[mcp_servers.a]\n"), ["a"]);
    let inside = "note = \"\"\"\n[mcp_servers.fake]\n\"\"\"\n[mcp_servers.real]\n";
    assert_eq!(names(inside), ["real"]);
    let literal = "note = \'\'\'\n[mcp_servers.fake]\n\'\'\'\nmcp_servers.real.x = 1\n";
    assert_eq!(names(literal), ["real"]);
    let escaped = "note = \"\"\"\na \\\"\"\" b\n[mcp_servers.fake]\n\"\"\"\n";
    assert!(names(escaped).is_empty());
    assert!(!scan_mcp_tables("a = \"\"\"x\n[mcp_servers]\nmcp_servers = 1\n\"\"\"\n").root_inline);
}

#[test]
fn quoted_names_with_escapes_unescape() {
    assert_eq!(names("[mcp_servers.\"a\\\\b\"]\n"), ["a\\b"]);
    assert_eq!(names("[mcp_servers.\"a\\u00e9b\"]\n"), ["aéb"]);
    assert_eq!(names("[mcp_servers.\"a\\tb\"]\n"), ["a\tb"]);
}

#[test]
fn debug_output_hides_args_urls_and_codex_values() {
    let mut s = stdio("s", "c");
    s.args = vec!["--token=LEAK-ARG-1".to_string()];
    s.env = pairs(&[("LOG_LEVEL", "LEAK-ENV-2")]);
    let dbg = format!("{s:?}");
    assert!(!dbg.contains("LEAK"), "{dbg}");

    let mut h = http(
        "h",
        "https://user:LEAK-URL-3@h/p?api_key=LEAK-URL-4#LEAK-URL-5",
    );
    h.headers = pairs(&[("X-A", "LEAK-HDR-6")]);
    let dbg = format!("{h:?}");
    assert!(!dbg.contains("LEAK"), "{dbg}");
    assert!(dbg.contains("api_key=<redacted>"), "{dbg}");

    let mut cs = stdio("c", "cmd");
    cs.args = vec!["--secret=LEAK-ARG-7".to_string()];
    cs.env = pairs(&[("MODE", "LEAK-ENV-8")]);
    let body = ok(codex(&cs));
    let dbg = format!("{body:?}");
    assert!(!dbg.contains("LEAK"), "{dbg}");
    let mut ch = http("ch", "https://h/p?token=LEAK-URL-9");
    ch.headers = pairs(&[("X-A", "LEAK-HDR-10")]);
    let dbg = format!("{:?}", ok(codex(&ch)));
    assert!(!dbg.contains("LEAK"), "{dbg}");
}
