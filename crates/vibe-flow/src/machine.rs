//! The state machine: a pure loader plus queries (mirrors `machine.mjs`).
//!
//! No transition enforcement lives here; `check_transition` is F7's job.

use std::path::Path;

use serde_json::Value;

/// A loaded `state-machine.json`. Queries are own-key lookups only, so a
/// state key spelled like an inherited member resolves to nothing.
#[derive(Debug, Clone)]
pub struct Machine {
    value: Value,
}

impl Machine {
    /// Reads `<data_dir>/state-machine.json`. A missing, unreadable,
    /// unparseable or non-object document is an `Err` naming the reason.
    pub fn load(data_dir: &Path) -> Result<Machine, String> {
        let path = data_dir.join("state-machine.json");
        let show = path.display();
        let bytes = std::fs::read(&path).map_err(|err| format!("cannot read {show}: {err}"))?;
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|err| format!("cannot parse {show}: {err}"))?;
        if !value.is_object() {
            return Err(format!(
                "state-machine.json at {show} is not a valid machine document (top-level {})",
                json_kind(&value)
            ));
        }
        Ok(Machine { value })
    }

    /// The raw machine document.
    pub fn value(&self) -> &Value {
        &self.value
    }

    fn states(&self) -> Option<&serde_json::Map<String, Value>> {
        self.value.get("states").and_then(Value::as_object)
    }

    /// The state's entry, or `None` when the key names no state.
    pub fn state_of(&self, key: &str) -> Option<&Value> {
        self.states().and_then(|states| states.get(key))
    }

    /// Every state key, sorted (matches `jq`'s `keys`).
    pub fn state_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .states()
            .map(|states| states.keys().cloned().collect())
            .unwrap_or_default();
        keys.sort_unstable();
        keys
    }

    /// The state's legal `next` states, string entries only.
    pub fn next(&self, key: &str) -> Vec<String> {
        self.state_of(key)
            .and_then(|state| state.get("next"))
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The state's linked skill (`null` when the state owns its orders).
    pub fn skill(&self, key: &str) -> Option<&Value> {
        self.state_of(key).and_then(|state| state.get("skill"))
    }

    /// The state's inline inject, when it has one.
    pub fn inject(&self, key: &str) -> Option<&Value> {
        self.state_of(key).and_then(|state| state.get("inject"))
    }

    /// Whether `from > to` is a gated edge.
    pub fn gate(&self, from: &str, to: &str) -> bool {
        self.value
            .get("gates")
            .and_then(Value::as_object)
            .is_some_and(|gates| gates.contains_key(&format!("{from}>{to}")))
    }

    /// Every gated edge key, sorted.
    pub fn gates(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .value
            .get("gates")
            .and_then(Value::as_object)
            .map(|gates| gates.keys().cloned().collect())
            .unwrap_or_default();
        keys.sort_unstable();
        keys
    }
}

/// Splits `<flow>.<phase>` on the FIRST dot; a single token maps flow == phase.
pub fn split_target(target: &str) -> (String, String) {
    match target.find('.') {
        Some(dot) => (target[..dot].to_string(), target[dot + 1..].to_string()),
        None => (target.to_string(), target.to_string()),
    }
}

fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(true) => "true",
        Value::Bool(false) => "false",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{Machine, split_target};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "vibe-flow-machine-{name}-{}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn shipped() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../flow")
    }

    fn shipped_machine() -> Machine {
        Machine::load(&shipped()).expect("load the shipped machine")
    }

    #[test]
    fn the_shipped_machine_has_thirteen_states() {
        let machine = shipped_machine();
        assert_eq!(machine.state_keys().len(), 13);
        assert_eq!(
            machine.state_keys(),
            [
                "feature.compound",
                "feature.design",
                "feature.impl",
                "feature.plan",
                "feature.verify",
                "idle",
                "quick.fix",
                "quick.triage",
                "quick.verify",
                "setup.apply",
                "setup.detect",
                "strategy.brainstorm",
                "strategy.spec",
            ]
        );
    }

    #[test]
    fn the_shipped_machine_has_thirty_legal_edges() {
        let machine = shipped_machine();
        let edges: usize = machine
            .state_keys()
            .iter()
            .map(|key| machine.next(key).len())
            .sum();
        assert_eq!(edges, 30);
    }

    #[test]
    fn exactly_two_edges_are_gated() {
        let machine = shipped_machine();
        assert_eq!(
            machine.gates(),
            [
                "feature.plan>feature.impl".to_string(),
                "feature.verify>feature.compound".to_string()
            ]
        );
        assert!(machine.gate("feature.plan", "feature.impl"));
        assert!(machine.gate("feature.verify", "feature.compound"));
        assert!(!machine.gate("feature.impl", "feature.plan"));
    }

    #[test]
    fn skill_and_inject_read_through() {
        let machine = shipped_machine();
        assert_eq!(machine.skill("idle"), Some(&serde_json::Value::Null));
        assert_eq!(
            machine.skill("feature.impl"),
            Some(&serde_json::json!("vibe"))
        );
        assert!(machine.inject("feature.impl").is_some_and(|v| v.is_null()));
        assert!(
            machine
                .inject("idle")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|s| s.starts_with("state=idle"))
        );
    }

    #[test]
    fn inherited_keys_are_not_states() {
        let machine = shipped_machine();
        for key in ["constructor", "toString", "__proto__", "hasOwnProperty"] {
            assert!(machine.state_of(key).is_none(), "{key}");
            assert!(machine.next(key).is_empty(), "{key}");
        }
    }

    #[test]
    fn split_target_splits_on_the_first_dot_only() {
        assert_eq!(
            split_target("feature.impl"),
            ("feature".to_string(), "impl".to_string())
        );
        assert_eq!(
            split_target("idle"),
            ("idle".to_string(), "idle".to_string())
        );
        assert_eq!(split_target("a.b.c"), ("a".to_string(), "b.c".to_string()));
        assert_eq!(split_target(".x"), (String::new(), "x".to_string()));
    }

    #[test]
    fn a_non_object_machine_is_refused() {
        let dir = scratch("non-object");
        for (name, body) in [
            ("null", "null"),
            ("array", "[]"),
            ("string", "\"x\""),
            ("number", "7"),
        ] {
            fs::write(dir.join("state-machine.json"), body).expect("write machine");
            let err = Machine::load(&dir).expect_err(name);
            assert!(err.contains("state-machine.json at"), "{name}: {err}");
        }
        fs::write(dir.join("state-machine.json"), "{ \"states\": ").expect("write machine");
        let err = Machine::load(&dir).expect_err("truncated");
        assert!(err.starts_with("cannot parse "), "{err}");
    }

    #[test]
    fn a_missing_machine_is_refused() {
        let dir = scratch("missing");
        let err = Machine::load(&dir).expect_err("missing machine");
        assert!(err.starts_with("cannot read "), "{err}");
    }
}
