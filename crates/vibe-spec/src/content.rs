//! validate's content checks (SF3, SF8–SF12, SF14, I1): ports of the awk
//! and grep blocks in `oracles/bash/spec/scripts/validate.sh`, rule order
//! and all. F17 fixes their known bugs (R-ID prefix, RFC-2119 boundary,
//! Scope scoping); F16 reproduces them.
use std::collections::BTreeSet;
use std::path::Path;

use crate::pattern::{cross_feature_deps, is_unit_heading, is_unit_row, r_ids, unit_ids};
use crate::text::{
    contains, contains_ci, first_line, has_line_starting, is_space, lines, name_of, sub_dirs,
    trim_space,
};
use crate::validate::Report;

const TOKEN_GROUPS: [&[u8]; 5] = [
    b"colors",
    b"typography",
    b"rounded",
    b"spacing",
    b"components",
];

fn flush<'a>(pending: &mut Option<&'a [u8]>, has_child: &mut bool, found: &mut Vec<&'a [u8]>) {
    if let Some(p) = pending.take()
        && !*has_child
        && TOKEN_GROUPS.contains(&p)
    {
        found.push(p);
    }
    *has_child = false;
}

/// awk `is_token_key`: `^[a-z]+:` naming a token group with nothing after the colon.
fn token_key(line: &[u8]) -> Option<&[u8]> {
    let n = line.iter().take_while(|c| c.is_ascii_lowercase()).count();
    if n == 0 || line.get(n) != Some(&b':') {
        return None;
    }
    let key = &line[..n];
    let rest = line.get(n + 1..).unwrap_or_default();
    (TOKEN_GROUPS.contains(&key) && trim_space(rest).is_empty()).then_some(key)
}

/// SF3: empty design-token groups in the frontmatter. No END rule: a
/// frontmatter that never closes flushes nothing.
pub fn design_tokens(r: &mut Report, text: &[u8], label: &[u8]) {
    if first_line(text) != b"---" {
        return;
    }
    let mut delim = 0;
    let mut pending: Option<&[u8]> = None;
    let mut has_child = false;
    let mut found: Vec<&[u8]> = Vec::new();
    for line in lines(text) {
        if line == b"---" {
            delim += 1;
            if delim == 2 {
                flush(&mut pending, &mut has_child, &mut found);
                break;
            }
            continue;
        }
        if delim != 1 {
            continue;
        }
        match line.first() {
            Some(&c) if !is_space(c) && c != b'#' => {
                flush(&mut pending, &mut has_child, &mut found);
                if let Some(key) = token_key(line) {
                    pending = Some(key);
                }
            }
            Some(&c) if is_space(c) && pending.is_some() => has_child = true,
            _ => {}
        }
    }
    for group in found {
        r.yellow(&cat!(label, ": empty token group '", group, "'"));
    }
}

/// SF8: the feature product's Scope section.
pub fn feature_scope(r: &mut Report, text: &[u8], label: &[u8]) {
    if !has_line_starting(text, b"## Scope") {
        r.yellow(&cat!(
            label,
            ": missing '## Scope' section (add Owns / Does not own table)"
        ));
        return;
    }
    if !contains_ci(text, b"owns") {
        r.yellow(&cat!(label, ": Scope section missing 'Owns' column or row"));
    }
    if !contains_ci(text, b"does not own") {
        r.yellow(&cat!(
            label,
            ": Scope section missing 'Does not own' column or row"
        ));
    }
}

/// SF9: feature frontmatter fields (`sibling:` for product and tech only).
pub fn feature_frontmatter(r: &mut Report, text: &[u8], label: &[u8], kind: &str) {
    if first_line(text) != b"---" {
        return;
    }
    if !has_line_starting(text, b"feature:") {
        r.yellow(&cat!(label, ": missing 'feature:' in frontmatter"));
    }
    if !has_line_starting(text, b"parent:") {
        r.yellow(&cat!(label, ": missing 'parent:' in frontmatter"));
    }
    if (kind == "product" || kind == "tech") && !has_line_starting(text, b"sibling:") {
        r.yellow(&cat!(label, ": missing 'sibling:' in frontmatter"));
    }
}

/// `toupper(s) ~ /(SHALL|MUST|SHOULD|MAY)/` (substring; F17 adds the word boundary).
fn rfc(s: &[u8]) -> bool {
    let up = s.to_ascii_uppercase();
    [&b"SHALL"[..], b"MUST", b"SHOULD", b"MAY"]
        .iter()
        .any(|k| contains(&up, k))
}

/// SF10: `### Requirement:` blocks need an RFC-2119 keyword and a scenario.
/// A requirement with an empty title is never reported (the oracle's
/// `req_line != ""` guard).
pub fn requirements_scenarios(r: &mut Report, text: &[u8], label: &[u8]) {
    if !has_line_starting(text, b"### Requirement:") {
        return;
    }
    let mut issues: Vec<Vec<u8>> = Vec::new();
    let mut req: Vec<u8> = Vec::new();
    let mut has_rfc = false;
    let mut scenarios = 0;
    let mut in_req = false;
    let finish = |issues: &mut Vec<Vec<u8>>, req: &[u8], has_rfc: bool, scenarios: usize| {
        if req.is_empty() {
            return;
        }
        if !has_rfc {
            issues.push(cat!(
                "requirement missing RFC-2119 keyword (SHALL|MUST|SHOULD|MAY) -> ",
                req
            ));
        }
        if scenarios == 0 {
            issues.push(cat!("requirement has zero #### Scenario: blocks -> ", req));
        }
    };
    for line in lines(text) {
        if let Some(rest) = line.strip_prefix(b"### Requirement:") {
            finish(&mut issues, &req, has_rfc, scenarios);
            let title = rest
                .iter()
                .position(|&c| !is_space(c))
                .map_or(&b""[..], |i| &rest[i..]);
            req = title.to_vec();
            has_rfc = rfc(&req);
            scenarios = 0;
            in_req = true;
        } else if line.starts_with(b"#### Scenario:") {
            if in_req {
                scenarios += 1;
            }
        } else if line.starts_with(b"### ") || line.starts_with(b"## ") {
            if in_req {
                finish(&mut issues, &req, has_rfc, scenarios);
                in_req = false;
                req.clear();
                has_rfc = false;
                scenarios = 0;
            }
        } else if in_req && rfc(line) {
            has_rfc = true;
        }
    }
    if in_req {
        finish(&mut issues, &req, has_rfc, scenarios);
    }
    for issue in &issues {
        r.yellow(&cat!(label, ": ", issue));
    }
    if !issues.is_empty() {
        r.yellow(&cat!(label, ": requirement hint — use '### Requirement: Title SHALL ...' with one or more '#### Scenario: ...' blocks"));
    }
}

/// SF11: a feature plan needs unit headings and a Requirements Trace.
pub fn plan_structure(r: &mut Report, text: &[u8], label: &[u8]) {
    if !lines(text).iter().any(|l| is_unit_heading(l)) {
        r.yellow(&cat!(
            label,
            ": plan missing '### <feature>/<n>' unit sections (e.g. '### vibe-flow/1 — ...')"
        ));
    }
    if !contains_ci(text, b"requirements trace") {
        r.yellow(&cat!(
            label,
            ": plan missing 'Requirements Trace' section linking R-IDs to units"
        ));
    }
}

/// SF11: unit detail inside a root-plan milestone. The oracle prints these
/// from `awk | while read`, a subshell, so they are never counted.
pub fn root_plan_milestones(r: &mut Report, text: &[u8], label: &[u8]) {
    let mut in_milestone = false;
    for line in lines(text) {
        if line.starts_with(b"## M") && line.get(4).is_some_and(u8::is_ascii_digit) {
            in_milestone = true;
            continue;
        }
        if line.starts_with(b"## ") {
            in_milestone = false;
        }
        if in_milestone && is_unit_heading(line) {
            r.yellow_uncounted(&cat!(
                label,
                ": milestone section embeds unit heading (move to feature plan) -> ",
                line
            ));
        }
        if in_milestone && is_unit_row(line) {
            r.yellow_uncounted(&cat!(
                label,
                ": milestone section embeds unit table row (move to feature plan) -> ",
                line
            ));
        }
    }
}

/// The R-IDs of a feature product: lines of the Requirements Trace section,
/// and each `### Requirement:` block, in the oracle's awk rule order.
fn product_r_ids(text: &[u8]) -> BTreeSet<Vec<u8>> {
    let mut ids = BTreeSet::new();
    let mut emit = |s: &[u8]| {
        for id in r_ids(s) {
            ids.insert(id.to_vec());
        }
    };
    let mut in_trace = false;
    let mut in_req = false;
    let mut buf: Vec<u8> = Vec::new();
    for line in lines(text) {
        if line
            .to_ascii_lowercase()
            .starts_with(b"## requirements trace")
        {
            in_trace = true;
            continue;
        }
        if line.starts_with(b"## ") {
            in_trace = false;
        }
        if in_trace {
            emit(line);
            continue;
        }
        if line.starts_with(b"### Requirement:") {
            if in_req {
                emit(&buf);
            }
            in_req = true;
            buf = line.to_vec();
        } else if line.starts_with(b"### ") || line.starts_with(b"## ") {
            if in_req {
                emit(&buf);
                in_req = false;
                buf.clear();
            }
        } else if in_req {
            buf.push(b'\n');
            buf.extend_from_slice(line);
        }
    }
    if in_req {
        emit(&buf);
    }
    ids
}

/// SF12: duplicate unit IDs, and product R-IDs not cited in the plan
/// (`grep -q` substring: `R1` counts as cited by `R10`).
pub fn id_traceability(r: &mut Report, product: &Path, plan: &[u8], label: &[u8]) {
    let mut ids: Vec<&[u8]> = lines(plan)
        .into_iter()
        .filter(|l| is_unit_heading(l))
        .flat_map(unit_ids)
        .collect();
    ids.sort();
    let mut dups: Vec<&[u8]> = Vec::new();
    for pair in ids.windows(2) {
        if pair[0] == pair[1] && dups.last() != Some(&pair[0]) {
            dups.push(pair[0]);
        }
    }
    for dup in dups {
        r.yellow(&cat!(
            label,
            ": duplicate unit heading ID '",
            dup,
            "' in plan"
        ));
    }
    if !product.is_file() {
        return;
    }
    let ids = product_r_ids(&std::fs::read(product).unwrap_or_default());
    if ids.is_empty() {
        r.info(&cat!(label, ": no R-IDs in product.md — R-ID trace check skipped (tag requirement headings '(R1)' to enable)"));
        return;
    }
    for id in ids {
        if !contains(plan, &id) {
            r.yellow(&cat!(
                label,
                ": ",
                id,
                " in product.md not cited in plan.md (light traceability check)"
            ));
        }
    }
}

/// I1: unit references to other features in a feature plan.
///
/// Deliberate divergence (pinned): the oracle interpolates the feature name
/// into an ERE (`a.b` also filters `a-b/2`; a name like `a(b` makes its grep
/// fail and print nothing). The port matches the name literally.
pub fn cross_deps(r: &mut Report, plan: &[u8], feature: &str, label: &[u8]) {
    let own = |d: &[u8]| {
        d.strip_prefix(feature.as_bytes())
            .and_then(|rest| rest.strip_prefix(b"/"))
            .is_some_and(|n| !n.is_empty() && n.iter().all(u8::is_ascii_digit))
    };
    let deps: BTreeSet<&[u8]> = lines(plan)
        .into_iter()
        .flat_map(cross_feature_deps)
        .filter(|d| !own(d))
        .collect();
    for dep in deps {
        r.yellow(&cat!(
            label,
            ": cross-feature unit dependency '",
            dep,
            "' — use a whole-feature gate in the root plan Feature Sequence, not a unit edge"
        ));
    }
}

/// awk `norm`: drop `` ` `` and `*`, trim, ASCII-lowercase.
fn norm(s: &[u8]) -> Vec<u8> {
    let kept: Vec<u8> = s
        .iter()
        .copied()
        .filter(|&c| c != b'`' && c != b'*')
        .collect();
    trim_space(&kept).to_ascii_lowercase()
}

/// The Owns items of one feature product, in the oracle's order.
fn owns_items(text: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut column = false;
    for line in lines(text) {
        let starts = line.iter().position(|&c| !is_space(c));
        if starts.and_then(|i| line.get(i)) != Some(&b'|') {
            column = false;
            continue;
        }
        let fields: Vec<&[u8]> = line.split(|&c| c == b'|').collect();
        let raw3 = fields.get(2).copied().unwrap_or_default();
        let c2 = norm(fields.get(1).copied().unwrap_or_default());
        let c3 = norm(raw3);
        if c2 == b"owns" && c3 == b"does not own" {
            column = true;
            continue;
        }
        if c2 == b"owns" {
            out.extend(
                raw3.split(|&c| c == b',')
                    .map(norm)
                    .filter(|x| x.len() >= 5),
            );
            continue;
        }
        if c2 == b"does not own" {
            column = false;
            continue;
        }
        if c2.is_empty() || c2.iter().all(|&c| c == b'-') {
            continue;
        }
        if column && c2.len() >= 5 {
            out.push(c2);
        }
    }
    out
}

/// SF14: two features owning the same item.
pub fn sf14(r: &mut Report, dir: &Path) {
    let mut seen: Vec<(Vec<u8>, String)> = Vec::new();
    for fdir in sub_dirs(&dir.join("features")) {
        let product = fdir.join("product.md");
        if !product.is_file() {
            continue;
        }
        let feature = name_of(&fdir);
        for item in owns_items(&std::fs::read(&product).unwrap_or_default()) {
            if item.is_empty() {
                continue;
            }
            match seen.iter().find(|(i, _)| *i == item) {
                Some((_, other)) if *other != feature => {
                    r.yellow(&cat!(
                        "SF14: scope conflict — '",
                        other,
                        "' and '",
                        feature,
                        "' both own: ",
                        item
                    ));
                }
                Some(_) => {}
                None => seen.push((item, feature.clone())),
            }
        }
    }
}
