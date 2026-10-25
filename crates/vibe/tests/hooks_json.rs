#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The shared SessionStart hook entry helpers.
use std::path::Path;
use vibe::hooks_json::{add_hook, has_hook, hook_command, remove_hook};
use vibe::paths::Harness;
use vibe_core::json::{self, Value};

const CMD: &str = "\"/bin/vibe\" hook session-start --harness codex";

fn doc(text: &str) -> Value {
    json::parse(text).unwrap()
}

#[test]
fn hook_command_shape() {
    let c = hook_command(Path::new("/opt/v ibe/vibe"), Harness::Droid).unwrap();
    assert_eq!(c, "\"/opt/v ibe/vibe\" hook session-start --harness droid");
}

#[test]
fn hook_command_rejects_quote_in_path() {
    assert!(hook_command(Path::new("/o\"pt/vibe"), Harness::Claude).is_err());
}

#[test]
fn hook_command_rejects_shell_active_characters() {
    for bad in ["/o$pt/vibe", "/o`pt/vibe", "/o\npt/vibe", "/o\rpt/vibe"] {
        assert!(
            hook_command(Path::new(bad), Harness::Claude).is_err(),
            "{bad:?}"
        );
    }
    #[cfg(unix)]
    assert!(hook_command(Path::new("/o\\pt/vibe"), Harness::Claude).is_err());
    assert!(hook_command(Path::new("/o pt/v-i_b.e"), Harness::Claude).is_ok());
}

#[test]
fn add_hook_idempotent() {
    let mut d = doc("{}");
    add_hook(&mut d, CMD).unwrap();
    let once = json::to_compact(&d);
    add_hook(&mut d, CMD).unwrap();
    assert_eq!(json::to_compact(&d), once);
    assert_eq!(
        once,
        format!(
            "{{\"hooks\":{{\"SessionStart\":[{{\"matcher\":\"startup|resume|clear\",\"hooks\":[{{\"type\":\"command\",\"command\":{},\"timeout\":10}}]}}]}}}}",
            json::to_compact(&Value::str(CMD))
        )
    );
    assert!(has_hook(&d, CMD));
}

#[test]
fn add_hook_keeps_foreign_hooks() {
    let mut d = doc(
        r#"{"model":"x","hooks":{"Stop":[{"hooks":[]}],"SessionStart":[{"matcher":"startup","hooks":[{"type":"command","command":"other"}]}]}}"#,
    );
    add_hook(&mut d, CMD).unwrap();
    let groups = d
        .path(&["hooks", "SessionStart"])
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(groups.len(), 2);
    assert!(d.path(&["hooks", "Stop"]).is_some());
    assert!(d.path(&["model"]).is_some());
}

#[test]
fn remove_hook_keeps_foreign_hooks_in_shared_group() {
    let mut d = doc(&format!(
        "{{\"hooks\":{{\"SessionStart\":[{{\"hooks\":[{{\"type\":\"command\",\"command\":\"other\"}},{{\"type\":\"command\",\"command\":{}}}]}}]}}}}",
        json::to_compact(&Value::str(CMD))
    ));
    remove_hook(&mut d, CMD).unwrap();
    assert!(!has_hook(&d, CMD));
    assert_eq!(
        json::to_compact(&d),
        r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"other"}]}]}}"#
    );
}

#[test]
fn remove_hook_prunes_empty_containers_only() {
    let mut d = doc("{}");
    add_hook(&mut d, CMD).unwrap();
    remove_hook(&mut d, CMD).unwrap();
    assert_eq!(json::to_compact(&d), "{}");

    let mut d = doc(r#"{"hooks":{"Stop":[]}}"#);
    add_hook(&mut d, CMD).unwrap();
    remove_hook(&mut d, CMD).unwrap();
    assert_eq!(json::to_compact(&d), r#"{"hooks":{"Stop":[]}}"#);

    let mut d = doc(r#"{"hooks":{"SessionStart":[{"hooks":[]}]}}"#);
    remove_hook(&mut d, CMD).unwrap();
    assert_eq!(
        json::to_compact(&d),
        r#"{"hooks":{"SessionStart":[{"hooks":[]}]}}"#
    );

    let mut d = doc(r#"{"hooks":{}}"#);
    remove_hook(&mut d, CMD).unwrap();
    assert_eq!(json::to_compact(&d), r#"{"hooks":{}}"#);
}

#[test]
fn add_and_remove_report_change_and_wrong_types() {
    let mut d = doc("{}");
    assert_eq!(add_hook(&mut d, CMD), Ok(true));
    assert_eq!(add_hook(&mut d, CMD), Ok(false));
    assert_eq!(remove_hook(&mut d, CMD), Ok(true));
    assert_eq!(remove_hook(&mut d, CMD), Ok(false));

    for bad in [r#"{"hooks":[]}"#, r#"{"hooks":{"SessionStart":{}}}"#, "[]"] {
        let mut d = doc(bad);
        let before = json::to_compact(&d);
        assert!(add_hook(&mut d, CMD).is_err(), "{bad}");
        assert_eq!(json::to_compact(&d), before);
    }
    let mut d = doc(r#"{"hooks":{"SessionStart":{}}}"#);
    assert!(remove_hook(&mut d, CMD).is_err());
}
