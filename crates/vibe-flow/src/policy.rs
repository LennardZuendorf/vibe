//! The flow write policy: the fnmatch dialect of `policy.mjs`, as data.
//!
//! `match` entries are literal paths or globs (`*` = any run, `?` = any one
//! character; both cross `/`, matching bash's own case patterns). A literal
//! additionally matches any path-boundary suffix of the queried path, which
//! reproduces the guard's `PATTERN|*/PATTERN` idiom. Exact rules are consulted
//! across every rule before any glob rule. Arms are read top to bottom and the
//! first one admitting the queried state wins; `{state}` in a reason is
//! replaced with the queried state. Loading never fails: an absent, malformed
//! or refused file degrades to no rules and reports through `errors`.

use std::path::Path;

use serde_json::Value;

/// A rule verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The write is allowed.
    Allow,
    /// The write is allowed with a warning.
    Warn,
    /// The write is blocked.
    Block,
}

impl Verdict {
    /// The verdict's wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Allow => "allow",
            Verdict::Warn => "warn",
            Verdict::Block => "block",
        }
    }

    fn parse(text: &str) -> Option<Verdict> {
        match text {
            "allow" => Some(Verdict::Allow),
            "warn" => Some(Verdict::Warn),
            "block" => Some(Verdict::Block),
            _ => None,
        }
    }
}

/// The states an arm admits.
#[derive(Debug, Clone, PartialEq)]
pub enum States {
    /// The `"*"` catch-all.
    CatchAll,
    /// An explicit list; an empty list admits no state.
    List(Vec<String>),
}

/// One arm of a rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Arm {
    /// The states this arm admits.
    pub states: States,
    /// The verdict for those states.
    pub verdict: Verdict,
    /// The reason, with `{state}` left for interpolation.
    pub reason: String,
}

/// One write-policy rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// The rule id.
    pub id: String,
    /// Literal and glob patterns.
    pub patterns: Vec<String>,
    /// Ordered arms.
    pub arms: Vec<Arm>,
}

/// A loaded rule set.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Policy {
    /// The usable rules, in file order.
    pub rules: Vec<Rule>,
}

/// A load result: the usable rules plus every defect reported.
#[derive(Debug, Clone, Default)]
pub struct Loaded {
    /// The usable rules.
    pub policy: Policy,
    /// One line per defect.
    pub errors: Vec<String>,
}

/// The answer for one path and state.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    /// The verdict.
    pub verdict: Verdict,
    /// The interpolated reason.
    pub reason: String,
    /// The rule that answered, when one did.
    pub rule_id: Option<String>,
}

/// The policy file's path relative to a data home.
pub const POLICY_RELPATH: &str = "content/policy.json";

const CATCH_ALL: &str = "*";
const STATE_TOKEN: &str = "{state}";

/// Reads and validates `<data_dir>/content/policy.json`. Never fails.
pub fn load_policy(data_dir: &Path) -> Loaded {
    let mut errors = Vec::new();
    let path = data_dir.join("content").join("policy.json");
    let show = path.display();
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Loaded::default(),
        Err(err) => {
            errors.push(format!("policy: {show} is not readable as JSON ({err})"));
            return Loaded {
                policy: Policy::default(),
                errors,
            };
        }
    };
    let raw: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(err) => {
            errors.push(format!("policy: {show} is not readable as JSON ({err})"));
            return Loaded {
                policy: Policy::default(),
                errors,
            };
        }
    };
    let Some(object) = raw.as_object() else {
        errors.push(format!("policy: {show} is not a JSON object"));
        return Loaded {
            policy: Policy::default(),
            errors,
        };
    };
    if let Some(version) = object.get("version")
        && !is_version_one(version)
    {
        errors.push(format!(
            "policy: {show} declares version {}, but this engine only understands version 1 — refusing to load its rules",
            json_text(version)
        ));
        return Loaded {
            policy: Policy::default(),
            errors,
        };
    }
    let Some(Value::Array(entries)) = object.get("rules") else {
        errors.push(format!("policy: {show} has no 'rules' array"));
        return Loaded {
            policy: Policy::default(),
            errors,
        };
    };
    let mut rules = Vec::new();
    for (idx, entry) in entries.iter().enumerate() {
        let Some(rule) = entry.as_object() else {
            errors.push(format!("policy: rules[{idx}] is not an object"));
            continue;
        };
        let id = rule
            .get("id")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .unwrap_or("")
            .to_string();
        let patterns = normalize_match(rule.get("match"));
        if id.is_empty() {
            errors.push(format!("policy: rules[{idx}] is missing an 'id'"));
            continue;
        }
        if patterns.is_empty() {
            errors.push(format!("policy: rule '{id}' has no usable 'match' path"));
            continue;
        }
        let arms = normalize_arms(rule.get("arms"), &id, &mut errors);
        if arms.is_empty() {
            errors.push(format!("policy: rule '{id}' has no usable 'arms'"));
            continue;
        }
        rules.push(Rule { id, patterns, arms });
    }
    Loaded {
        policy: Policy { rules },
        errors,
    }
}

fn is_version_one(value: &Value) -> bool {
    value
        .as_u64()
        .map(|n| n == 1)
        .or_else(|| value.as_i64().map(|n| n == 1))
        .or_else(|| value.as_f64().map(|n| n == 1.0))
        .unwrap_or(false)
}

/// `normalizeMatchField`: a string, or the strings of an array; empties dropped.
fn normalize_match(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_string)
            .collect(),
        Some(Value::String(text)) if !text.is_empty() => vec![text.clone()],
        _ => Vec::new(),
    }
}

fn normalize_arms(value: Option<&Value>, id: &str, errors: &mut Vec<String>) -> Vec<Arm> {
    let Some(Value::Array(list)) = value else {
        errors.push(format!("policy: rule '{id}' has no 'arms' array"));
        return Vec::new();
    };
    let mut arms = Vec::new();
    for (idx, entry) in list.iter().enumerate() {
        let Some(arm) = entry.as_object() else {
            errors.push(format!("policy: rule '{id}' arms[{idx}] is not an object"));
            continue;
        };
        let states = match arm.get("states") {
            Some(Value::String(text)) if text == CATCH_ALL => States::CatchAll,
            Some(Value::Array(list)) => States::List(
                list.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
            ),
            _ => States::List(Vec::new()),
        };
        let verdict = match arm.get("verdict") {
            Some(value) => match value.as_str().and_then(Verdict::parse) {
                Some(verdict) => verdict,
                None => {
                    errors.push(format!(
                        "policy: rule '{id}' arms[{idx}] has an unknown verdict '{}' — defaulting to 'block'",
                        json_text(value)
                    ));
                    Verdict::Block
                }
            },
            None => Verdict::Block,
        };
        let reason = arm
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        arms.push(Arm {
            states,
            verdict,
            reason,
        });
    }
    arms
}

/// The JSON spelling of a value, as JS string interpolation would render it.
fn json_text(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// Answers `allow|warn|block` for writing `rel_path` in `state`.
pub fn decide(policy: &Policy, rel_path: &str, state: &str) -> Decision {
    if policy.rules.is_empty() {
        return unmatched();
    }
    let normalized = if rel_path.is_empty() {
        String::new()
    } else {
        normalize_path(rel_path)
    };
    let state_key = if state.is_empty() { "idle" } else { state };
    let suffixes = boundary_suffixes(&normalized);
    for rule in &policy.rules {
        if matches_exact(rule, &suffixes) {
            return rule_verdict(rule, state_key);
        }
    }
    for rule in &policy.rules {
        if matches_glob(rule, &suffixes) {
            return rule_verdict(rule, state_key);
        }
    }
    unmatched()
}

fn unmatched() -> Decision {
    Decision {
        verdict: Verdict::Allow,
        reason: String::new(),
        rule_id: None,
    }
}

fn rule_verdict(rule: &Rule, state: &str) -> Decision {
    for arm in &rule.arms {
        if admits(&arm.states, state) {
            return Decision {
                verdict: arm.verdict,
                reason: arm.reason.replace(STATE_TOKEN, state),
                rule_id: Some(rule.id.clone()),
            };
        }
    }
    Decision {
        verdict: Verdict::Allow,
        reason: String::new(),
        rule_id: Some(rule.id.clone()),
    }
}

fn admits(states: &States, state: &str) -> bool {
    match states {
        States::CatchAll => true,
        States::List(list) => list.iter().any(|entry| entry == state),
    }
}

fn has_glob(pattern: &str) -> bool {
    pattern.contains(['*', '?'])
}

fn matches_exact(rule: &Rule, suffixes: &[String]) -> bool {
    rule.patterns
        .iter()
        .any(|pattern| !has_glob(pattern) && suffixes.iter().any(|suffix| suffix == pattern))
}

fn matches_glob(rule: &Rule, suffixes: &[String]) -> bool {
    rule.patterns.iter().any(|pattern| {
        has_glob(pattern) && suffixes.iter().any(|suffix| glob_match(pattern, suffix))
    })
}

/// Every boundary suffix of a normalized path, including the whole path.
fn boundary_suffixes(path: &str) -> Vec<String> {
    let segments: Vec<&str> = path.split('/').collect();
    (0..segments.len())
        .map(|start| segments[start..].join("/"))
        .collect()
}

/// `path.posix.normalize`, minus a leading `./` run, never a bare dot.
pub fn normalize_path(path: &str) -> String {
    let normalized = posix_normalize(path);
    let trimmed = strip_leading_dot_slashes(&normalized);
    if trimmed == "." {
        String::new()
    } else {
        trimmed
    }
}

fn strip_leading_dot_slashes(path: &str) -> String {
    let mut rest = path;
    while let Some(next) = rest.strip_prefix("./") {
        rest = next;
    }
    rest.to_string()
}

/// The POSIX normalization Node's `path.posix.normalize` performs.
fn posix_normalize(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let is_absolute = path.starts_with('/');
    let trailing = path.ends_with('/');
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.last().is_some_and(|last| *last != "..") {
                    segments.pop();
                } else if !is_absolute {
                    segments.push("..");
                }
            }
            other => segments.push(other),
        }
    }
    let body = segments.join("/");
    if body.is_empty() {
        if is_absolute {
            return "/".to_string();
        }
        return if trailing {
            "./".to_string()
        } else {
            ".".to_string()
        };
    }
    let body = if trailing { format!("{body}/") } else { body };
    if is_absolute {
        format!("/{body}")
    } else {
        body
    }
}

/// One token of a parsed glob pattern.
enum GlobToken {
    /// Zero or more UTF-16 code units, none a JS line terminator.
    Star,
    /// Exactly one UTF-16 code unit, not a JS line terminator.
    Any,
    /// An exact run of UTF-16 code units.
    Literal(Vec<u16>),
}

/// Parses by `char` (a JS `for...of` iterates code points), so an astral
/// literal becomes its surrogate pair. Runs of `*` collapse first, exactly as
/// the oracle collapses `/\*+/g` before translating.
fn glob_tokens(pattern: &str) -> Vec<GlobToken> {
    let mut tokens = Vec::new();
    let mut previous_star = false;
    for ch in pattern.chars() {
        if ch == '*' {
            if !previous_star {
                tokens.push(GlobToken::Star);
            }
            previous_star = true;
            continue;
        }
        previous_star = false;
        if ch == '?' {
            tokens.push(GlobToken::Any);
            continue;
        }
        let mut buffer = [0u16; 2];
        tokens.push(GlobToken::Literal(ch.encode_utf16(&mut buffer).to_vec()));
    }
    tokens
}

/// The four line terminators JS `.` refuses to match without the `s` flag.
fn is_line_terminator(unit: u16) -> bool {
    matches!(unit, 0x000A | 0x000D | 0x2028 | 0x2029)
}

/// Whether `pattern` matches `path` in full, with `*` spanning `/`.
///
/// Matching is over UTF-16 code units, mirroring the oracle's `new RegExp`
/// without the `u` flag: `?` is one code unit, and neither wildcard crosses a
/// line terminator. Memoized on failed positions, so a hostile pattern costs
/// `pattern length * path length * path length` at worst, never exponential
/// backtracking.
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern = glob_tokens(pattern);
    let path: Vec<u16> = path.encode_utf16().collect();
    let mut failed = vec![false; (pattern.len() + 1) * (path.len() + 1)];
    matches_at(&pattern, 0, &path, 0, &mut failed)
}

fn matches_at(
    pattern: &[GlobToken],
    pi: usize,
    path: &[u16],
    si: usize,
    failed: &mut [bool],
) -> bool {
    let key = pi * (path.len() + 1) + si;
    if failed[key] {
        return false;
    }
    let hit = step(pattern, pi, path, si, failed);
    if !hit {
        failed[key] = true;
    }
    hit
}

fn step(pattern: &[GlobToken], pi: usize, path: &[u16], si: usize, failed: &mut [bool]) -> bool {
    match pattern.get(pi) {
        None => si == path.len(),
        Some(GlobToken::Star) => {
            for next in si..=path.len() {
                if next > si && is_line_terminator(path[next - 1]) {
                    break;
                }
                if matches_at(pattern, pi + 1, path, next, failed) {
                    return true;
                }
            }
            false
        }
        Some(GlobToken::Any) => {
            matches!(path.get(si), Some(&unit) if !is_line_terminator(unit))
                && matches_at(pattern, pi + 1, path, si + 1, failed)
        }
        Some(GlobToken::Literal(units)) => {
            let end = si.saturating_add(units.len());
            path.get(si..end) == Some(units.as_slice())
                && matches_at(pattern, pi + 1, path, end, failed)
        }
    }
}

/// The `{{invariants}}` prose, one line per rule.
pub fn render_invariants(policy: &Policy) -> String {
    policy
        .rules
        .iter()
        .map(|rule| {
            let arms = rule
                .arms
                .iter()
                .map(arm_display)
                .collect::<Vec<_>>()
                .join("; ");
            format!("- {} — {arms}", match_display(&rule.patterns))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn match_display(patterns: &[String]) -> String {
    patterns
        .iter()
        .map(|pattern| format!("`{pattern}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn states_display(states: &States) -> String {
    match states {
        States::CatchAll => "otherwise".to_string(),
        States::List(list) if list.is_empty() => "(no state)".to_string(),
        States::List(list) => list
            .iter()
            .map(|state| format!("`{state}`"))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn arm_display(arm: &Arm) -> String {
    let reason = if arm.verdict != Verdict::Allow && !arm.reason.is_empty() {
        format!(
            " — {}",
            arm.reason.replace(STATE_TOKEN, "the current state")
        )
    } else {
        String::new()
    };
    format!(
        "{}: {}{reason}",
        states_display(&arm.states),
        arm.verdict.as_str()
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{Verdict, decide, glob_match, load_policy, normalize_path, render_invariants};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "vibe-flow-policy-{name}-{}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn shipped() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../flow")
    }

    #[test]
    fn normalize_matches_the_node_truth_table() {
        let cases: &[(&str, &str)] = &[
            ("", ""),
            (".", ""),
            ("./", ""),
            ("./a", "a"),
            ("a/b/", "a/b/"),
            ("a//b", "a/b"),
            ("a/./b", "a/b"),
            ("a/../b", "b"),
            ("../a", "../a"),
            ("a/b/../", "a/"),
            ("/a/b", "/a/b"),
            ("a/b/.", "a/b"),
            ("a/", "a/"),
            ("/", "/"),
            ("//", "/"),
            ("a//", "a/"),
            ("././a", "a"),
            ("..", ".."),
            ("a/b/../../c", "c"),
            ("/..", "/"),
            ("a/../../b", "../b"),
        ];
        for (input, want) in cases {
            assert_eq!(normalize_path(input), *want, "normalize({input:?})");
        }
    }

    #[test]
    fn glob_crosses_slashes_and_questions_match_one_utf16_unit() {
        let cases: &[(&str, &str, bool)] = &[
            ("*", "", true),
            ("*", "a/b/c", true),
            (".planning/*", ".planning/a", true),
            (".planning/*", ".planning/a/b", true),
            (".planning/*", "x/.planning/a", false),
            ("src/*", "src/a/b", true),
            ("?.md", "a.md", true),
            ("?.md", "/.md", true),
            ("?.md", "\n.md", false),
            ("*a*a*a*a*b", "a", false),
            ("*a*a*a*a*b", "aaaab", true),
            ("a*b", "a/b", true),
            ("docs/**", "docs/a/b", true),
        ];
        for (pattern, path, want) in cases {
            assert_eq!(
                glob_match(pattern, path),
                *want,
                "glob({pattern:?}, {path:?})"
            );
        }
    }

    #[test]
    fn a_question_measures_utf16_code_units_not_bytes() {
        // é is one UTF-16 unit (two Rust bytes); 😀 is a surrogate pair.
        assert!(glob_match("?.md", "é.md"));
        assert!(!glob_match("?.md", "😀.md"));
        assert!(!glob_match("?", "😀"));
        assert!(glob_match("??", "😀"));
    }

    #[test]
    fn a_literal_astral_character_matches_its_surrogate_pair() {
        assert!(glob_match("😀", "😀"));
        assert!(glob_match("a😀b", "a😀b"));
        assert!(glob_match("*😀*", "x😀y"));
        assert!(!glob_match("a?b", "a😀b"));
    }

    #[test]
    fn wildcards_do_not_match_js_line_terminators() {
        for terminator in ['\n', '\r', '\u{2028}', '\u{2029}'] {
            let text = terminator.to_string();
            assert!(!glob_match("?", &text), "? matched {terminator:?}");
            assert!(!glob_match("*", &text), "* matched {terminator:?}");
            let wrapped = format!("a{terminator}b");
            assert!(!glob_match("a?b", &wrapped), "a?b matched {terminator:?}");
            assert!(!glob_match("a*b", &wrapped), "a*b matched {terminator:?}");
            assert!(glob_match(&wrapped, &wrapped), "literal {terminator:?}");
        }
    }

    #[test]
    fn a_hostile_pattern_finishes_quickly() {
        let start = std::time::Instant::now();
        assert!(!glob_match("*a*a*a*a*a*a*b", &"a".repeat(200)));
        assert!(!glob_match(
            "*a*a*a*a*a*a*a*a*a*a*b",
            &("ab".repeat(100) + "c")
        ));
        let elapsed = start.elapsed();
        assert!(elapsed.as_secs() < 2, "took {elapsed:?}");
    }

    #[test]
    fn an_absent_policy_loads_empty_without_errors() {
        let loaded = load_policy(&scratch("absent"));
        assert!(loaded.policy.rules.is_empty());
        assert!(loaded.errors.is_empty());
    }

    #[test]
    fn the_shipped_policy_loads_without_errors_and_eleven_rules() {
        let loaded = load_policy(&shipped());
        assert_eq!(loaded.errors, Vec::<String>::new());
        assert_eq!(loaded.policy.rules.len(), 11);
    }

    #[test]
    fn the_shipped_render_is_the_expected_prose() {
        let loaded = load_policy(&shipped());
        let prose = render_invariants(&loaded.policy);
        let lines: Vec<&str> = prose.lines().collect();
        assert_eq!(lines.len(), 11);
        assert_eq!(
            lines[0],
            "- `.agents/skills/vibe/state.json`, `.vibe/run/flow/state.json` — otherwise: block — state.json is written only via set-state.sh, never by direct edit"
        );
        assert!(
            lines[1].contains("otherwise: block")
                && lines[1].ends_with("(current: the current state)"),
            "{}",
            lines[1]
        );
        assert_eq!(
            lines[9],
            "- `CLAUDE.md`, `AGENTS.md` — otherwise: warn — CLAUDE.md/AGENTS.md active-rules block is generated by regen-active-rules.sh; edits inside the markers are overwritten next compound"
        );
        assert!(lines[10].contains(": allow"), "{}", lines[10]);
        assert!(!prose.contains("{state}"), "uninterpolated token: {prose}");
    }

    #[test]
    fn decide_answers_the_shipped_rules() {
        let loaded = load_policy(&shipped());
        let policy = &loaded.policy;
        let lessons = policy.rules[1].patterns[0].clone();
        let root_spec = policy.rules[3].patterns[0].clone();
        let frozen = policy.rules[6].patterns[0].clone();
        assert_eq!(
            decide(policy, ".agents/skills/vibe/state.json", "idle").verdict,
            Verdict::Block
        );
        assert_eq!(
            decide(policy, &lessons, "feature.compound").verdict,
            Verdict::Allow
        );
        assert_eq!(decide(policy, &lessons, "idle").verdict, Verdict::Block);
        assert_eq!(
            decide(policy, &root_spec, "strategy.spec").verdict,
            Verdict::Allow
        );
        assert_eq!(decide(policy, &root_spec, "idle").verdict, Verdict::Block);
        assert_eq!(
            decide(policy, "docs/spec/tech.md", "idle").verdict,
            Verdict::Block
        );
        assert_eq!(
            decide(policy, ".planning/x", "idle").verdict,
            Verdict::Allow
        );
        assert_eq!(
            decide(policy, ".planning/x", "setup.apply").verdict,
            Verdict::Block
        );
        assert_eq!(
            decide(policy, &frozen, "feature.impl").verdict,
            Verdict::Warn
        );
        assert_eq!(
            decide(policy, "src/main.rs", "feature.verify").verdict,
            Verdict::Warn
        );
        assert_eq!(
            decide(policy, "src/main.rs", "feature.impl").verdict,
            Verdict::Allow
        );
        assert_eq!(decide(policy, "AGENTS.md", "idle").verdict, Verdict::Warn);
        assert_eq!(
            decide(policy, "unmatched/path.txt", "idle").verdict,
            Verdict::Allow
        );
    }

    #[test]
    fn the_reason_carries_the_queried_state() {
        let loaded = load_policy(&shipped());
        let lessons = loaded.policy.rules[1].patterns[0].clone();
        let decision = decide(&loaded.policy, &lessons, "feature.impl");
        assert_eq!(decision.verdict, Verdict::Block);
        assert!(
            decision.reason.contains("(current: feature.impl)"),
            "{}",
            decision.reason
        );
        assert_eq!(decision.rule_id.as_deref(), Some("lessons"));
    }

    #[test]
    fn an_exact_rule_outranks_a_glob_rule_whatever_the_order() {
        // The shipped `docs/spec/lessons.md` rule is exact; the `docs/spec/*`
        // shape is exercised with a hand-built policy so the divergence from
        // the bash oracle's top-to-bottom read stays pinned.
        let dir = scratch("exact-first");
        fs::create_dir_all(dir.join("content")).expect("content dir");
        fs::write(
            dir.join("content").join("policy.json"),
            r#"{"rules":[
              {"id":"glob","match":"docs/*","arms":[{"states":"*","verdict":"block","reason":"glob"}]},
              {"id":"exact","match":"docs/lessons.md","arms":[{"states":"*","verdict":"allow","reason":""}]}
            ]}"#,
        )
        .expect("write policy");
        let loaded = load_policy(&dir);
        assert!(loaded.errors.is_empty());
        let decision = decide(&loaded.policy, "docs/lessons.md", "idle");
        assert_eq!(decision.verdict, Verdict::Allow);
        assert_eq!(decision.rule_id.as_deref(), Some("exact"));
        assert_eq!(
            decide(&loaded.policy, "docs/other.md", "idle")
                .rule_id
                .as_deref(),
            Some("glob")
        );
    }

    #[test]
    fn an_arm_that_admits_no_state_allows_rather_than_falling_through() {
        let dir = scratch("no-admitting-arm");
        fs::create_dir_all(dir.join("content")).expect("content dir");
        fs::write(
            dir.join("content").join("policy.json"),
            r#"{"rules":[
              {"id":"narrow","match":"x.txt","arms":[{"states":["setup.apply"],"verdict":"block","reason":"narrow"}]},
              {"id":"wide","match":"x.txt","arms":[{"states":"*","verdict":"block","reason":"wide"}]}
            ]}"#,
        )
        .expect("write policy");
        let loaded = load_policy(&dir);
        assert!(loaded.errors.is_empty());
        // The first matching rule has no admitting arm, so the answer is
        // allow — the second rule is NOT consulted.
        assert_eq!(
            decide(&loaded.policy, "x.txt", "idle").verdict,
            Verdict::Allow
        );
        assert_eq!(
            decide(&loaded.policy, "x.txt", "setup.apply").verdict,
            Verdict::Block
        );
    }

    #[test]
    fn per_rule_defects_are_isolated_and_reported() {
        let dir = scratch("defects");
        fs::create_dir_all(dir.join("content")).expect("content dir");
        fs::write(
            dir.join("content").join("policy.json"),
            r#"{"rules":[
              {"match":"a","arms":[{"states":"*","verdict":"block"}]},
              {"id":"no-arms","match":"b"},
              {"id":"bad-verdict","match":"c","arms":[{"states":"*","verdict":"nope"}]},
              {"id":"keep","match":"d","arms":[{"states":"*","verdict":"warn","reason":"r"}]}
            ]}"#,
        )
        .expect("write policy");
        let loaded = load_policy(&dir);
        assert_eq!(
            loaded.errors,
            vec![
                "policy: rules[0] is missing an 'id'".to_string(),
                "policy: rule 'no-arms' has no 'arms' array".to_string(),
                "policy: rule 'no-arms' has no usable 'arms'".to_string(),
                "policy: rule 'bad-verdict' arms[0] has an unknown verdict 'nope' — defaulting to 'block'".to_string(),
            ]
        );
        let ids: Vec<&str> = loaded
            .policy
            .rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect();
        assert_eq!(ids, ["bad-verdict", "keep"]);
    }

    #[test]
    fn a_version_other_than_one_refuses_every_rule() {
        let dir = scratch("version");
        fs::create_dir_all(dir.join("content")).expect("content dir");
        let path = dir.join("content").join("policy.json");
        fs::write(
            &path,
            r#"{"version":2,"rules":[{"id":"a","match":"x","arms":[{"states":"*","verdict":"block"}]}]}"#,
        )
        .expect("write policy");
        let loaded = load_policy(&dir);
        assert!(loaded.policy.rules.is_empty());
        assert_eq!(loaded.errors.len(), 1);
        assert_eq!(
            loaded.errors[0],
            format!(
                "policy: {} declares version 2, but this engine only understands version 1 — refusing to load its rules",
                path.display()
            )
        );
    }

    #[test]
    fn a_missing_rules_array_is_reported() {
        let dir = scratch("no-rules");
        fs::create_dir_all(dir.join("content")).expect("content dir");
        fs::write(dir.join("content").join("policy.json"), "{}").expect("write policy");
        let loaded = load_policy(&dir);
        assert!(loaded.policy.rules.is_empty());
        assert_eq!(loaded.errors.len(), 1);
        assert!(
            loaded.errors[0].ends_with("has no 'rules' array"),
            "{}",
            loaded.errors[0]
        );
    }

    #[test]
    fn a_non_object_policy_is_reported() {
        let dir = scratch("non-object");
        fs::create_dir_all(dir.join("content")).expect("content dir");
        fs::write(dir.join("content").join("policy.json"), "[]").expect("write policy");
        let loaded = load_policy(&dir);
        assert!(loaded.policy.rules.is_empty());
        assert!(
            loaded.errors[0].ends_with("is not a JSON object"),
            "{}",
            loaded.errors[0]
        );
    }
}
