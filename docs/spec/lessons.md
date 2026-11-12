# Lessons

Mistakes made and rules to prevent repeating them. Written during `compound`,
read on entry to `*.design` and `*.triage` so past mistakes shape new work.
Tags make entries retrievable — scan for tags matching the work in hand.

### Spec strictness: warn-first, then migrate
**Pattern:** New validate.sh checks shipped as errors immediately; dogfood repo's legacy feature plans failed validation before migration, blocking the harness from validating itself.
**Rule:** Ship structural validators warn-first; promote warn→error only after live specs are migrated during compound. Pair every validator with a behaviour test in `spec/tests/run.sh`.
**Tags:** spec, validate, templates, dogfood
**Date:** 2026-06-06

### Single-source the per-turn orders; don't duplicate them in the machine
**Pattern:** The per-state inject orders lived in BOTH `state-machine.json` (`inject` strings) and the `vibe-*` skill bodies. Two copies of the same orders drift apart, and nothing flags the divergence.
**Rule:** Author the orders once, in the linked skill, as a machine-extractable `<!-- vibe:orders:<state> -->` block; carry `inject: null` in the machine for skill-owning states (only `idle` keeps an inline fallback); resolve via `orders.sh` (cursor → `skill` link → block → interpolate `<feature>` only, so the prompt cache stays byte-stable). Hooks are thin shells over that resolver.
**Tags:** vibe-flow, d12, inject, prompt-cache, single-source
**Date:** 2026-06-18

### A Claude Code plugin cannot bundle skills outside ./skills/
**Pattern:** vibe's skills are canonically under `.agents/skills/`, but the plugin manifest has no `skills` path field and bans `../` in component paths (`commands`/`agents`/`hooks`/`mcpServers` only). Pointing the plugin at `.agents/skills/` is impossible.
**Rule:** Let the *plugin* carry only the Claude-specific runtime wiring it uniquely provides (`commands` + `hooks` via `${CLAUDE_PLUGIN_ROOT}`); deliver the platform-neutral core (the `spec`/`vibe` skill and `.agents/skills/vibe`) as project files through `install.sh`. "Single install" = run the installer. Keeps adapters thin and the core canonical.
**Tags:** platform-adapters, plugin, install, claude-code, adapters
**Date:** 2026-06-18

### An installer must preserve per-project runtime state across a re-copy
**Pattern:** `install.sh` copied the core then unconditionally `rm`'d the target's `.agents/skills/vibe/state.json` and re-seeded from the template, silently resetting a live mid-flow cursor to idle on every re-run — while advertising "idempotent". The cursor is the one file that is per-project runtime state, not managed core.
**Rule:** A provisioner that refreshes managed files must snapshot per-project runtime state (the flow cursor) before the copy and restore it after, seeding only when genuinely absent. "Idempotent" has to hold for *user* state, not just managed files; pin it with a regression test that a live cursor (`feature.impl <feature>`) survives a re-install.
**Tags:** platform-adapters, install, idempotency, cursor, state
**Date:** 2026-06-20

### Marker-bounded merge must validate marker pairing before rewriting
**Pattern:** `merge-agents.sh` entered its replace path on a substring `grep` for both markers regardless of order; a file with the markers reversed (end before start) made the awk silently drop all trailing user content — breaking the script's own "never touch content outside the markers" guarantee.
**Rule:** Any tool that rewrites a region between managed markers must confirm the markers exist as exact lines AND that start precedes end before mutating — refuse (never mangle) on reversed/overlapping markers, and always write via temp + atomic rename. Pair it with a test that a reversed-marker file is left byte-untouched.
**Tags:** agent-instructions, merge, markers, content-safety, atomic
**Date:** 2026-06-20

### Skill design: promote superpowers proactively, remain self-sufficient
**Pattern:** When a skill can delegate to superpowers or subagents, two failure modes appear: (1) the skill silently self-executes every step, never surfacing that a better executor exists — users never learn the tools; (2) the skill hard-couples to superpowers and fails or degrades silently when they're unavailable.
**Rule:** Skills SHOULD proactively offer their optimal executor at each step — "I can use X for this, want me to?" — and MUST self-execute from their constraint documents if the user declines or the executor is unavailable. The order is always: offer first, self-suffice second. Never silently skip the offer; never block on the answer.
**Tags:** spec, superpowers, subagents, skill-design, interoperability
**Date:** 2026-06-21
**Superseded:** 2026-10-13 by D45 — feature design/plans now default to superpowers; the spec skill routes, it does not offer.

### Script self-location: search for markers, don't count hops
**Pattern:** `orders.sh` and `regen-active-rules.sh` resolved the repo root with a fixed number of `dirname` hops tuned to the old `.agents/skills` nesting; after the `flow/` move the same scripts silently hit generic fallbacks or overshot the root when invoked via their canonical path — the symlinked invocation still worked, masking the breakage.
**Rule:** Scripts reachable through compat symlinks must locate the repo root by upward marker search (`docs/spec`/`.git`), never fixed hop counts; pin with path-parity tests asserting byte-identical output via both real and symlinked invocation.
**Tags:** monorepo-split, symlinks, self-location, path-parity, prompt-cache
**Date:** 2026-07-03

### Uninstall must surgically invert the install into shared dirs, and the test must discriminate
**Pattern:** `install.sh` copies vibe files into *shared* dirs (`.claude/commands`, `.claude/hooks`) the user may also populate. A naïve `rm -rf "$dir"` uninstall would take the user's co-located files with it. The first cut removed the right files but the uninstall tests only asserted that *shipped* files were gone — a review found that swapping the surgical `remove_shipped` for `rm -rf` still passed every assertion (a false-negative on a data-loss path).
**Rule:** An uninstaller must delete only the paths the installer created (per-file inverse of the copy), never blanket-remove a shared directory; pruning *emptied* dirs is fine. Pair every preservation guarantee with a **discriminating** test — one that fails if the safety code is replaced by the naïve destructive version (drop a user file into each shared dir, run uninstall, assert it survives *and* the shipped file is gone). Reuse the tested marker-pairing guard for the managed instruction block; never re-implement it.
**Tags:** install-tooling, uninstall, data-safety, shared-dirs, discriminating-tests
**Date:** 2026-07-03

### The dogfood repo is a privileged target — eval on a fresh, non-git install
**Pattern:** `orders.sh` (the flow's headline per-turn feature) worked in every in-repo test and on the source repo, but a README-only stranger eval installing into a fresh `mktemp -d` found it silently returned `state=unknown` for every state. Root cause: it located the skills dir by searching upward for a `.spec`/`.git` marker, and a fresh install target has neither — while the source repo (and the dogfood repo the tests run in) always does. The bug was invisible precisely because every test environment was privileged. <!-- literal -->
**Rule:** A tool that will be *installed elsewhere* must be tested from a representative fresh target (a bare `mktemp -d`, no `.git`, no `docs/spec`), not just the source/dogfood repo. Prefer self-location relative to the script's own path over repo-root markers the target may lack. Run a periodic "stranger" eval (fresh agent, docs-only, throwaway sandbox) as a release gate — it exercises the install-target reality the in-repo suites cannot.
**Tags:** release-docs, stranger-eval, self-location, install-target, dogfood, orders
**Date:** 2026-07-03

### Compound is where drift is born — enforce it mechanically
**Pattern:** flow-mvp shipped and merged (PR #14) with every unit still marked NOT STARTED in its own `plan.md`, no root `plan.md` Feature Sequence row, and no compound at all — the feature folder was never archived and its Delivered note never written. The gates object, a quick-flow compound state, the precedence section, the evidence-receipt verify tooth, and caveman demotion were all demonstrably live in the tree, yet the `.spec/` memory said the work had never begun. End-of-feature discipline is exactly the moment attention lapses (the work "feels done"), and nothing in the harness forced the compound — the audit caught it weeks later. <!-- literal -->
**Rule:** Compound must be mechanically enforced, not trusted to discipline. A drift check (`spec/scripts/check-drift.sh`, CI-wired after `validate.sh`) fails when a directory under `docs/spec/features/` has no row in the root `docs/spec/plan.md`, and flags any `NOT STARTED` unit left in a feature `plan.md`. Hand-written assertion counts are errored the same way — they rot silently. A green suite is not a compounded feature; make the missing-compound state impossible to merge past.
**Tags:** compound, drift, dogfood, ci
**Date:** 2026-07-09

### A single-source parity test must pin prose to the code, not to a sibling doc
**Pattern:** flow-legibility's write invariants live in three places: enforced in `detect-context.sh decide`, restated in the `<!-- vibe:doctrine -->` block, and rendered in the AGENTS.md template. The first "single-source" test only compared the doctrine block and the template *to each other*, and only on the union of state *names* — so it passed even when a rule was reassigned to the wrong state (e.g. lessons.md writable in a state `decide` actually blocks). Two prose copies agreeing proves nothing about the policy they both claim to describe; the code was never in the loop.
**Rule:** Any doc that restates a code-enforced policy must be parity-tested against the CODE, per rule, not against a sibling doc. Extract each rule's allow-set from the enforcer (`detect-context.sh decide`) and assert the prose's per-rule state set matches it exactly — a set-equality check, not a name-union. A prose↔prose test only catches copy-paste rot; it silently permits rule-reassignment drift, which is the failure that actually misleads an agent.
**Tags:** spec, single-source, discriminating-tests, doctrine, parity
**Date:** 2026-07-18

### Injected context is append-only — budget it and class it by trigger
**Pattern:** The flow's injection design treated hook output as if it were a refreshable status line. It is not: `UserPromptSubmit` stdout enters the conversation layer, which is append-only, so a byte-stable per-turn order accumulates one copy per turn for the whole session — and the `Stop` gate's stuck-phase predicate fired *every* non-idle turn, queueing a redundant `vibe-warn:` line into every subsequent prompt. Two adjacent errors came from the same wrong mental model: `doctrine.sh` appended a live `Cursor:` line to `SessionStart`, whose output is *replayed* rather than re-run on `--resume` (so it is stale by construction), and the write invariants were restated in seven prose locations while the guard already enforced them — against documented guidance that a bloated instruction file makes Claude "ignore your actual instructions".
**Rule:** Treat every injection channel as a context budget with an explicit line cap, and class payload by trigger: *level* (every turn — the minimum that must always be true), *edge* (only when the cursor changes — orders, contracts, retrieved lessons), *event* (only when something happened — drift, warnings). Never put live state on `SessionStart`; it replays stale on resume. Never state in always-on prose what a hook enforces — render it in the guard's verdict at the moment of violation, where it is transient, free, and actionable. Lint the budgets in CI; discipline will not hold them.
**Tags:** injection, context-budget, hooks, prompt-cache, doctrine, agents-md
**Date:** 2026-08-10

### A guard is only as strong as the capability it bans, not the spelling it matches
**Pattern:** The duplicate-primitive scan failed review three times running. Each round closed exactly the hole reported and left a smaller one of the same kind: it began matching three *spellings* of "re-derive a primitive path"; inverted to banning *ingredients* (filename literals, layout constants) — and that round exported a `cursorPath()` helper to avoid taking a waiver, creating a legal spelling of the forbidden thing; then banned the helper *identifiers* — and widened ingredients to multi-token regexes while the matcher still ran **per line**, so ordinary Prettier wrapping evaded. Separately, nothing scanned `engine/tests/`, so `await import('../tests/helpers.mjs')` reached a working duplicate cursor reader in one line. Every round believed it was done, and every round's fix was the previous round's next hole.
**Rule:** When a guard is evaded, do not add a pattern for the evasion — that buys exactly one round. Close the *capability*: state the invariant as "no module may obtain X by any means unless allowlisted", then find the mechanical property that makes it true regardless of syntax. Match against the whole comment-stripped file, never per line. Close the scanned set under whatever relation the attacker can traverse (here, module resolution — which covers static `import`, `import()`, `require`, and `createRequire` without naming any of them). Pin exemptions by exact line *and occurrence count*, never by file. And require the implementer to produce the list of evasions it tried against its **own** fix, including the ones that failed — the round that finally held was the first to produce that artifact.
**Pinned-by:** final whole-branch review, spec v0.4 phase 1 (2026-10-13)
**Tags:** js-core, discriminating-tests, static-analysis, guards, structural-closure, mutation-testing
**Date:** 2026-08-13

### A check that examines nothing must fail loudly, never pass quietly
**Pattern:** This feature produced the same shape five times. A test harness reported `ok` from a bare `return`. Thirty-one parity tests asserted nothing when `jq` was absent and counted as passes. The bash-3.2 lint enumerated **zero** files in a git tree with an empty index, and twelve nonexistent paths when its root was a repo subdirectory — passing in both, so a release tarball could be declared clean having scanned nothing. The jq half of the matrix could vanish entirely with both CI legs green. And while verifying a fix, the controller's own mutation regex matched a line a refactor had renamed: the suite ran unmutated and returned green, which looked exactly like a caught mutant.
**Rule:** Every guard must assert its own population — a floor on what it examined, and that the things it examined exist. Absence of findings is only evidence when presence of *input* is proven. Extend this to your own verification: after planting a mutant, confirm it actually landed (`git diff --numstat`) before believing the result, and prefer structural floors over hand-written counts, which rot. A green that cannot distinguish "checked and clean" from "checked nothing" is not a green.
**Tags:** js-core, vacuous-tests, guards, ci, mutation-testing, verification
**Date:** 2026-08-13

### Porting a script destroys the oracle that proves the port — freeze it first
**Pattern:** `runGuardHook` and `runGateHook` — the two hooks that can *block* a tool call or a turn — were ported by overwriting their bash originals in place. Every other command kept its oracle and was parity-tested against it; these two silently could not be, and no per-unit review noticed, because from inside each unit nothing was missing. The whole-branch review restored the oracles from `main` and found 30 real divergences: the engine blocked where bash did not, a latent block-loop that could wedge a session.
**Rule:** When replacing an implementation, copy its predecessor into the test tree as a frozen oracle **before** the first line of the replacement is written, and make differential comparison part of the suite. Prioritise by blast radius: anything that can block, delete, or halt gets its oracle frozen first. Where an oracle disagrees with *itself* across its own code paths, follow the fail-safe branch and pin the divergence with a control — losing one turn of enforcement is recoverable, wedging a session is not.
**Pinned-by:** final whole-branch review, spec v0.4 phase 1 (2026-10-13)
**Tags:** js-core, porting, parity, oracles, blocking-hooks, differential-testing
**Date:** 2026-08-13

### A control case that depends on an unset variable must delete it, not merely not set it
**Pattern:** The jq-half gate's "bare" spawn existed to prove that a run with a skipped jq half exits non-zero. `runCommand` builds its child env from `{...process.env}`, so on the one CI leg that runs the whole suite under `VIBE_NO_JQ=1`, the bare spawn inherited the opt-in and was not bare — it took the deliberately-skipped branch, exited 0, and failed its own assertion. The test's comment said isolation was handled "without touching this runner's own PATH": PATH was isolated, the environment never was. The same class had just been fixed one file over for `CLAUDE_PROJECT_DIR`.
**Rule:** A test whose meaning is "variable X is not set" must **delete** X from the child environment, not rely on the parent not having it — and the deletion belongs on the shared spawn helper so every call site can use it. Then run the suite under each ambient variable it reads, in both states, as its own leg. Related: run every CI leg locally, byte-exact, before pushing — the round that shipped this bug ran none of them, and the round that fixed it ran all ten.
**Tags:** js-core, hermeticity, test-isolation, environment, ci
**Date:** 2026-08-13

### A comment asserting a safety property becomes load-bearing — test it or delete it
**Pattern:** `install.sh` claimed "Source-only artifacts (co-located tests…) never ship" while scrubbing only `vibe/tests` and not `vibe/engine/tests`, so 8,313 lines of test code across 17 files landed in every user repo. The comment was not merely stale: the primitive scan's one exemption was *argued on that premise*, so a false comment was silently holding up a correctness argument. Across these rounds, four other comments were found asserting protections the code had just been proven not to provide — including two reason codes promising exactly the guarantee that had been demonstrated absent.
**Rule:** Treat a comment that asserts a safety or scope property as an untested claim, and either give it a test or delete it. When a fix proves a comment wrong, correct the comment in the same commit — a stale comment is worse than none, because the next reader (and the next reviewer's reasoning) will rely on it. Prefer assertions that are structural over lists of names: scrub every `tests/` directory at any depth and assert *no test artifact anywhere* reaches a target, rather than naming two directories that the next co-located suite will silently escape.
**Tags:** js-core, install, comments, false-invariants, packaging
**Date:** 2026-08-13

### Refresh the checkout map after a branch change
**Pattern:** After switching branches, I queried paths remembered from the previous tree; several had moved or no longer existed, so `rg` emitted avoidable path errors. The first branch switch also failed because this environment mounted `.git` read-only; the user-authorized retry needed an elevated Git write.
**Rule:** After a branch change or fast-forward, confirm the active ref and enumerate the current paths before running branch-specific commands. If a Git operation is blocked by the sandbox, use an authorized Git-write context instead of retrying the same command unchanged.
**Tags:** workflow, branches, sandbox, repo-navigation
**Date:** 2026-10-01

### A manifest merge must refresh what was redeployed, not what was selected
**Pattern:** The install manifest's merge dropped the records of every host in the `--platform` selection — but a selected host's deployment can be skipped (`--only spec` installs no host entry points), so a `--only spec --platform codex` re-install erased codex's records while its files stayed on disk; the next uninstall took the manifest pass, read the surviving records as the full inventory, and stranded vibe-owned files. The tempting broader fix — refresh only hosts present in this run's action list — was worse: a collision refusal correctly records nothing, so it would have re-admitted stale records and made a later uninstall delete user content.
**Rule:** An ownership manifest's merge must key its record-refresh on what THIS run actually redeployed, never on the selector and never on the action list: capture the deploy-skip decision (a preserve list the merge consults) and refresh only the rest. Pin the exact stranding scenario (install A+B, re-install one under a deploy-skipping combination, uninstall, assert nothing strands) and the collision corollary in the same pass.
**Tags:** install, manifest, ownership, idempotency, uninstall, data-safety
**Date:** 2026-10-02

<!-- Format for each lesson:
### [Short description]
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this
**Tags:** comma, separated, keywords
**Date:** YYYY-MM-DD
-->

### Replace a table row by its table, never by its leading cell
**Pattern:** A batched doc edit replaced plan row `| F3 | …` by matching the line prefix; the same prefix opened a row in the issue catalog table, which was overwritten with the 7-column plan row — two issue links lost and the table malformed. The check scripts only asserted the new text existed, so they passed.
**Rule:** Anchor a row replacement to its table (header or section) and assert afterwards that every table kept its column count and that the old row text is gone only where intended. A post-edit check must also look for collateral damage, not only for the new text.
**Tags:** spec, docs, plan, tables, batch-edit
**Date:** 2026-10-13

### A new shell script runs the umbrella suite, not just its own
**Pattern:** The pressure-test harness passed shellcheck and its own honesty checks, then broke the flow suite's bash-3.2 portability lint (`"${arr[@]}"` on an empty array under `set -u`). The implementer and two reviewers ran only the spec checks; the defect surfaced two tasks later.
**Rule:** Any change that adds or edits a tracked `*.sh` file — test scripts included — runs `bash tests/run.sh` (all suites) and `git ls-files '*.sh' | grep -Ev '^oracles/(bash|js)/' | xargs shellcheck` (the CI command) before commit; a phase gate later found two SC2015 hits in `spec/tests/run.sh` the same way. Guard array expansions as `${arr[@]+"${arr[@]}"}` for stock macOS bash 3.2.
**Tags:** bash, portability, bash32, testing, spec
**Date:** 2026-10-13

### Run wall-time budgets on a quiet machine before calling a regression
**Pattern:** The engine's 150 ms performance-budget test failed repeatedly while several agents and suites ran in parallel; alone, the slowest command measured 85 ms and the suite went 674/674.
**Rule:** Before treating a wall-time failure as a regression, rerun the suite alone and compare against its logged control. Never run timing suites concurrently with other suites in a gate.
**Tags:** testing, performance, flaky, engine
**Date:** 2026-10-13

### Plan text must fit the tests that already constrain it
**Pattern:** Two plan steps specified exact text that the repo's existing tests rejected: compound orders 435 bytes against a 400-byte per-block budget, and a fallback line that did not contain the plan's own assert string.
**Rule:** When a plan dictates exact content, check it against every existing budget, pin and assert that covers that file before the plan is approved — and make the plan's own asserts match its own text.
**Tags:** planning, orders, testing, superpowers
**Date:** 2026-10-13

### Sweep for the meaning, not one spelling of it
**Pattern:** A fix swept `flow/` for the literal `.spec/features/` to find every line that still made the fallback the default. Lines saying "follow the spec skill feature.md steps 1–4" and injecting "the feature's `product.md` requirements" carried the same meaning without the string and survived until the final review. <!-- literal -->
**Rule:** When retiring a default, list every phrasing that expresses it (paths, file names, step references, template names) and grep for all of them; then pin the invariant with a test that fails on any unqualified reference, not just the one spelling.
**Tags:** flow, spec, superpowers, grep, review
**Date:** 2026-10-13

### A finding marked must-fix is fixed in its own task
**Pattern:** A task review found a stale safety comment, marked it "must-fix", and parked it as a deferred minor for the final wave. The final whole-branch review then reported it as the branch's only Critical.
**Rule:** A finding that a binding rule makes mandatory (stale safety comment, test that asserts nothing) enters the task's fix loop; only genuinely optional polish is deferred to the final review.
**Tags:** review, sdd, process, comments
**Date:** 2026-10-13

### A headless CLI inside a read loop eats the loop's input
**Pattern:** The pressure harness ran `claude -p` inside `while read … done <scenarios.tsv`. The headless CLI reads stdin when it is not a terminal, so the first session would have swallowed the remaining scenario rows and the loop would have ended after one scenario. The bug shipped through two reviews and was caught only before the first live run.
**Rule:** Every command that may read stdin, when called inside a `while read` loop, gets `</dev/null`. Live harnesses also keep each session's transcript and stderr on disk; a silenced stderr hides auth and rate-limit failures.
**Tags:** bash, harness, testing, pressure, spec
**Date:** 2026-10-14

### A skill description must name the small work it covers
**Pattern:** After the rewrite, the spec skill passed every design-shaped pressure scenario but failed "Quick fix: … Just fix it" with and without superpowers, plus "Add a --verbose flag" without them (GREEN 9/12). The agent read the file and wrote straight away. Naming one-line fixes, typos, new flags, and "quick"/"just fix it" in the description took the run to 12/12 twice.
**Rule:** The description is the only text an agent sees before choosing. It names the smallest work it must catch, in the user's words, and says to invoke before reading files. A body row (Routing, Red Flags) does not catch what the description misses.
**Tags:** skills, spec, triggers, pressure, superpowers
**Date:** 2026-10-14

### A compiled, passing plan is not a proven guard
**Pattern:** Every Phase 2 code block was compiled and tested in a scratch workspace before the plan was written, yet task reviews found guards that pass without guarding in six of eight tasks: a schema validator that ignored `$ref` siblings, contracts that did not pin required sets, a vocabulary list missing half the state names, a path-dependency branch no test reached, a harness `cwd` that escaped the sandbox, and a boundary test with no decoy to stop.
**Rule:** For every guard a plan adds, plant the defect it exists to catch and watch it fail before the plan ships. "It compiles and the tests pass" proves the code runs, not that the check discriminates.
**Tags:** testing, planning, review, guards, rust, contracts
**Date:** 2026-10-14

### A backtracking matcher needs a bound before it reaches a hook
**Pattern:** The hand-written glob matcher backtracked without memoization: `*a*a*a*a*b` against 200 characters took 4.9 s, and six `**/` segments took 6.8 s. It was meant to run on every path inside a hook.
**Rule:** Any recursive wildcard matcher memoizes failed (pattern, subject) positions, and a test pins a hostile pattern under a generous time bound.
**Tags:** rust, performance, glob, hooks
**Date:** 2026-10-14

### Derive a banned vocabulary from its source, never from memory
**Pattern:** The core crate's "no tool vocabulary" test listed `feature.` and `strategy.` but missed `quick.*`, `setup.*` and the `agents-md` channel.
**Rule:** A guard against a set of names reads the set from the file that defines it (state machine, channel config) and asserts a floor on how many it derived.
**Tags:** testing, guards, rust, flow
**Date:** 2026-10-14

### Floors and fixtures must hold at every stage of a plan
**Pattern:** A source-file floor of 5 failed at the stage that had 3 files, and fixture files named `*.sh` broke the repo-wide CI shellcheck once tracked. Both surfaced only when intermediate stages were built and files were staged.
**Rule:** Build each intermediate task state of a plan, and dry-run the whole phase with every new file staged, before handing the plan over. Fixture files that are not real scripts never use a script extension.
**Tags:** planning, testing, ci, shellcheck
**Date:** 2026-10-14

### The write guard suffix-matches nested spec paths
**Pattern:** Corpus files under `tests/parity/corpus/**/tree/.spec/plan.md` are blocked for Edit/Write in every flow state, because the policy rule matches the `.spec/plan.md` suffix. <!-- literal -->
**Rule:** Never hand-edit generated corpus trees; change the generator and regenerate. A real fix belongs to the flow policy (anchor root-spec rules at the repo root).
**Tags:** flow, policy, parity, spec
**Date:** 2026-10-14

### Control a red timing gate against a clean base before blaming load or the branch
**Pattern:** The engine perf budget failed (`doctor` 194–252 ms > 150 ms). It was blamed first on load, then on `target/`. A clean `origin/main` export failed identically; the cost is `validate-state.sh` (81 ms, ~10 `jq` spawns) under `doctor`, independent of the branch.
**Rule:** When a timing gate fails, profile the slow command's subprocesses and rerun the same gate on a clean base export before attributing the cause.
**Tags:** testing, performance, verification
**Date:** 2026-10-14

### Enumerate the oracle's shell semantics before porting a line
**Pattern:** Task reviews of the F16 ports found five parity defects, each in a shell construct the port had read as plain logic:
- glob `*.md` matching dirs and dangling links;
- the raw `$@` tail including a leading `--`;
- `basename "$(dirname …)"`;
- bash arithmetic in `-gt`;
- a feature name interpolated into a `grep -E` pattern.
**Rule:** Before porting a bash line, name each construct's exact semantics (glob, argv tail, arithmetic, regex interpolation, coreutils formatting). Add a corpus case for each one. Mimic it, or pin the divergence on purpose with a reason.
**Tags:** rust, parity, oracle, bash, review
**Date:** 2026-10-15

### Line-based Markdown scanners must skip fenced code
**Pattern:** The F16 `plan` query counted `- [ ]` and `## ` lines inside fenced examples. This repo's own plans hold fenced headings and checkboxes, so a fully ticked task would report `in-progress` forever. The plan text mandated the defect.
**Rule:** Any line scanner over Markdown tracks ``` and ~~~ fences and ignores structure inside them. Test it against this repo's real plans, not only synthetic fixtures.
**Tags:** markdown, parsing, plans, testing
**Date:** 2026-10-15

### Reuse the path-display helper instead of rejoining components
**Pattern:** The `plan`/`feature` JSON rebuilt paths by joining `Path::components()` with `/`. For a spec root outside the project, the `RootDir` component produced `//Users/...`. `Root::display` already handled that case.
**Rule:** Render a path through one shared helper that handles both the inside-project and the outside-project case. Test with a `--root` outside the project.
**Tags:** rust, paths, json, contracts
**Date:** 2026-10-15

### Sandbox git repos in tests inherit the user's signing config
**Pattern:** Five `inject-triggers/6` adapter asserts failed while the SSH signing agent was down. The test's throwaway `git commit` in its sandbox inherited global `commit.gpgsign=true`, failed silently (`>/dev/null 2>&1`), and left staged files that the `git status --porcelain` assertions then saw.
**Rule:** Every test that runs `git init` in a sandbox sets `commit.gpgsign false` (and `tag.gpgsign false`) in that repo next to `user.email`/`user.name`, and never discards a setup commit's exit status.
**Tags:** testing, git, sandbox, environment
**Date:** 2026-10-15

### Concurrent tests that share a deterministic scratch dir race under load
**Pattern:** `common::project(name)` reused one directory per name; parallel tests clobbered each other.
**Rule:** Give every test fixture a unique directory (process id plus an atomic counter).
**Tags:** rust, tests, sandbox
**Date:** 2026-10-15

### Checks that read git history must detect shallow clones
**Pattern:** SF29 and `trace` read a depth-1 CI checkout as full history: every file got the boundary commit's date, and every unit looked uncited.
**Rule:** Check `rev-parse --is-shallow-repository` first, and stay silent or say so.
**Tags:** spec, validate, git
**Date:** 2026-10-15

### Never split free text on separator bytes; use NUL
**Pattern:** A `%x1f`/`%x1e` git log format broke on commit messages that contained those bytes.
**Rule:** Use `git log -z`, because git refuses NUL in messages, and validate each record's header (hex sha).
**Tags:** git, rust, parsing
**Date:** 2026-10-15

### Give each file one owning check per problem
**Pattern:** In layered trees, legacy SF20/SF21 errored on files SF30 already reported, and SF26/SF33 repeated SF20.
**Rule:** When a new layout or check covers a file, route that file away from the old check. Test that a bad input yields exactly one finding.
**Tags:** spec, validate, layered
**Date:** 2026-10-15

### "Path resolves" means is_file inside the project
**Pattern:** SF36 accepted `#anchor` (empty path resolves to the project dir) and directories, because it used `.exists()`.
**Rule:** Strip fragments before the emptiness check, then require `is_file()` and canonical containment.
**Tags:** spec, validate, paths
**Date:** 2026-10-15

### Fixtures must have the target's real shape
**Pattern:** A one-part layered fixture hid SF32 false duplicates that fire on every multi-part tree (customer-portal's target).
**Rule:** Model fixtures on the real target, including repeated structures.
**Tags:** spec, validate, layered, tests
**Date:** 2026-10-15

### A git pathspec prunes merge and empty commits
**Pattern:** Scoping `trace`'s `git log` with `-- .` at the repo top silently dropped merge and empty commits (history simplification), so their unit citations vanished.
**Rule:** Pass no pathspec when the scope is the whole repository. When a real sub-path scope is needed, accept or document that merges are pruned.
**Tags:** git, trace, rust
**Date:** 2026-10-15

### Normalise the base path, not just the target
**Pattern:** The layered link resolver applied `..` to the link target but compared it against an unnormalised `--root docs/../docs/spec`, so valid absolute links read as outside the root.
**Rule:** Normalise both sides of a path comparison lexically before `strip_prefix`/`starts_with`. Test with a root given through `..`.
**Tags:** spec, validate, paths, layered
**Date:** 2026-10-15

### Compare Windows paths in their plain form
**Pattern:** Test helpers built expected paths with `fs::canonicalize`, which returns `\\?\C:\…` on Windows, while the binary printed `C:\…`; four `root` tests failed only on the Windows CI legs.
**Rule:** Strip the verbatim prefix in test helpers, then build expected paths one component per `join` (a verbatim base silently turned `/` into `\`; a plain one does not). Push early so the Windows legs run.
**Tags:** rust, tests, windows, paths
**Date:** 2026-10-16

### Shellcheck versions disagree; write for the strictest
**Pattern:** `out="$(cd X && cmd || true)"` passed local shellcheck 0.11.0 but CI's runner shellcheck flagged SC2015 (info) and failed the job before any suite ran.
**Rule:** Never write `A && B || C` in shell; put `|| true` outside the command substitution. Treat a CI shellcheck finding as authoritative over the local version.
**Tags:** shell, ci, shellcheck
**Date:** 2026-10-16

### A status edit is a spec edit: bump `updated:`
**Pattern:** Closing the F2–F17 rows changed `.spec/plan.md` without touching its `updated:` field, so `vibe-spec validate` gained an SF29 warning (updated older than the last commit). <!-- literal -->
**Rule:** Bump `updated:` in the same commit as any edit to a spec doc, status cells included. Run `vibe-spec validate --json` before committing a spec edit.
**Tags:** spec, compound, sf29
**Date:** 2026-10-16

### A glob port sorts by the glob's words, not bare names
**Pattern:** `text::sub_dirs` sorted bare names; the bash oracle's `features/*/` glob sorts `name/`, so `spec-delta` precedes `spec`.
**Rule:** When porting a glob, sort by the glob's expanded words (including the trailing `/`) and pin a prefix-pair case.
**Tags:** rust, parity, sorting
**Date:** 2026-10-16

### Skipping is failing open
**Pattern:** `walk_files` skipped unreadable dirs (the pre-merge gate passed) and symlinked entries (import dropped requirements, exit 0); `unwrap_or_default` on a read turned an unreadable target into empty content that the write then truncated (data loss).
**Rule:** A read or walk error that feeds a gate or a write must abort, never default or skip silently. Test each with `chmod 000` and a symlink.
**Tags:** rust, spec, safety, io
**Date:** 2026-10-17

### Plan every write in memory
**Pattern:** Multi-file writes were not all-or-nothing (a second write failure left the first written), and `--dry-run` planned against disk instead of the merged bytes.
**Rule:** Plan every write in memory, keep originals, restore on failure. Dry-run executes the same plan and gates only the writes.
**Tags:** rust, spec, promote, atomicity
**Date:** 2026-10-17

### Lstat every path component before a delete
**Pattern:** `promote --done` deleted through a symlinked artifact directory (leaf-only lstat).
**Rule:** Before an irreversible delete, lstat every path component from the project root down and refuse any symlink.
**Tags:** rust, spec, safety, symlink
**Date:** 2026-10-18

### Probe root-discovering commands in their own spec tree
**Pattern:** A reviewer probe ran `import` in a scratch dir without `.spec`; root discovery walked up and imported into the repo's real `.spec/`. <!-- literal -->
**Rule:** Probes of root-discovering commands create their own `docs/spec` in the scratch tree, or run outside the repo.
**Tags:** spec, tests, discovery
**Date:** 2026-10-18

### Grep for scripted edits before claiming them
**Pattern:** A fix-round edit silently failed to apply (the search text did not match the rustfmt-formatted file) and the round reported done; the re-review caught the missing test.
**Rule:** After a scripted edit, grep for the new content before claiming it landed.
**Tags:** tooling, review
**Date:** 2026-10-16

### Check the signing agent before a commit batch
**Pattern:** Commits were blocked when the SSH signing agent (Bitwarden) was locked.
**Rule:** Run `ssh-add -l` before a commit batch. On failure leave the work staged and report; never disable signing.
**Tags:** git, signing
**Date:** 2026-10-17

### Compare file identity, not spelling, before merging two writes
**Pattern:** `promote` decided that a merge target and a delta target were one file by comparing their path text. The same file under two paths (a symlinked directory) got two writes, and the second overwrote the first while the command reported success.
**Rule:** Compare file identity (device and inode, or canonical path) before merging two writes to one file; fall back to the lexical path only when a file is missing.
**Tags:** rust, spec, safety, symlink
**Date:** 2026-10-18

### Scope hook paths by project identity
**Pattern:** `validate --file` compared raw payload paths, so `..` and cwd-relative paths returned an empty result without an error.
**Rule:** Normalise every hook path through one identity function and make it project-relative before filtering.
**Tags:** rust, spec, hooks
**Date:** 2026-10-19

### Write runtime markers atomically, never through links
**Pattern:** The session marker was written through a symlinked `.vibe` and a symlinked `spec-session`, so it truncated or created files outside the project.
**Rule:** `lstat` the directory and the file, skip on a symlink, and write with temp file plus rename.
**Tags:** rust, safety, symlink
**Date:** 2026-10-19

### Give every hook a timeout
**Pattern:** The spec hook entries had no `timeout`, while the flow entries had 10.
**Rule:** Set `"timeout"` on every hook entry in `settings.json` and generated `hooks.json`, and assert it in a test.
**Tags:** hooks, tooling
**Date:** 2026-10-19

### Route agents through the engine
**Pattern:** The `spec-promoter` agent edited root docs by hand and never ran `vibe-spec promote`, which applies merge blocks and deltas.
**Rule:** An agent that owns a workflow runs the engine command and works only the checklist it prints.
**Tags:** agents, spec
**Date:** 2026-10-19

### Match ownership markers by exact line and restore multi-file writes
**Pattern:** `setup` matched its ownership marker as a substring, so a foreign file could be overwritten. Its multi-file writes had no restore. The hooks path from a subdirectory skipped the symlink walk.
**Rule:** Match the marker as a whole line, make multi-file writes all-or-nothing, and make the hooks path absolute before the symlink walk. Normalise lexically: `git rev-parse --path-format=absolute` resolves symlinks and hides a symlinked hooks dir.
**Tags:** rust, safety, git
**Date:** 2026-10-19

### Probe in a dir with its own `git init`
**Pattern:** A reviewer probe ran in a plain subdirectory inside the repo. Git resolved the enclosing repo and the probe installed a hook into the shared git dir.
**Rule:** Create probe dirs outside the repo tree and run `git init` in each before any git-aware command.
**Tags:** tests, git, process
**Date:** 2026-10-19

### Prove RED per behaviour, not by stubbing the command
**Pattern:** Task 9 wrote the implementation before the tests and showed RED only by stubbing the whole command, which proves nothing about each behaviour.
**Rule:** Prove RED per behaviour by breaking that one behaviour and watching its test fail. Write tests first.
**Tags:** tdd, tests, process
**Date:** 2026-10-19

### A rewrite tool needs corruption-class tests
**Pattern:** The first `migrate` regex matched `..spec` and `.spec.ts`. Link rewriting corrupted angle-bracket targets and missed reference definitions, CRLF lines lost `\r`, duplicate D numbers silently became two files, and the walk descended into `target/` and `node_modules/`.
**Rule:** Before a tool rewrites user content, test each corruption class: look-alike tokens, link forms, line endings, duplicate keys and walk pruning. A committed rewrite cannot be undone.
**Tags:** rust, migrate, safety
**Date:** 2026-10-19

### Verify must check what the change introduced
**Pattern:** `migrate` verified only the literal count and ignored validate errors it had introduced.
**Rule:** After a mutating command, compare validation errors before and after and revert on an increase. Report pre-existing errors without failing on them.
**Tags:** rust, migrate, verify
**Date:** 2026-10-19

### Test the new arm of a dual-path change in isolation
**Pattern:** Dual-root shell changes had no test with only the new root present, so removing the new arm left the suite green. CI also installed the toolchain after the first cargo use, and a test script pointed at a legacy fixture that `migrate` would rewrite.
**Rule:** Give each dual-path change a test where only the new path exists. Install toolchains before the first step that needs them. Point tests at fixtures the migration excludes.
**Tags:** tests, ci, migrate
**Date:** 2026-10-19

### Re-run every gate check after the final edit
**Pattern:** The `migrate --check` gate ran before the last hand edit and the report claimed it passed. It exited 1 afterwards.
**Rule:** Run each gate command after the last edit and paste that output. A result from before the edit proves nothing.
**Tags:** verification, process
**Date:** 2026-10-19

### Audit rewritten prose for truth
**Pattern:** `migrate` rewrote historical facts in lessons and made "the old root today" prose false or duplicated.
**Rule:** After a bulk text rewrite, read the changed prose. Tag historical lines so a sweep leaves them alone.
**Tags:** migrate, docs
**Date:** 2026-10-19

### Marker removal must not consume frontmatter
**Pattern:** Removing inserted marker lines consumed the closing `---` of hub frontmatter, and the validator did not flag it.
**Rule:** Treat unclosed frontmatter as an error (SF20) and test marker removal on every doc kind.
**Tags:** spec, validate, frontmatter
**Date:** 2026-10-19

### Run `set-state strategy.spec` as its own command
**Pattern:** A failed command in a chain wrote spec files before `set-state strategy.spec` had run.
**Rule:** Run `set-state` alone and confirm its output before any command that writes spec files.
**Tags:** process, flow, spec
**Date:** 2026-10-19

### A hook shim must tell a block from a crash by output
**Pattern:** The shim passed every exit 2 through as a tooth block. A stale binary's usage error also exits 2, so it blocked every edit and every Stop. Per-task reviews missed it; only the whole-branch review caught it.
**Rule:** Pass a block through only when the output proves it was one, such as a single `blocked:` line. Map every other code to allow. Test with a fake binary for each event.
**Tags:** hooks, fail-open, review
**Date:** 2026-10-19

### Keep shipped prose root-neutral
**Pattern:** A bulk rewrite put the new root's path into the shipped skill. In a repo on the old root, following the skill created the new root and hid the user's tree.
**Rule:** Shipped instructions name "the spec root (`vibe-spec root`)", not a fixed path. Warn when both roots exist.
**Tags:** spec, migrate, docs, dual-root
**Date:** 2026-10-19

### Date test fixtures relative to today
**Pattern:** A freshness check moved to compare against today, and a fixture with a fixed date stopped passing.
**Rule:** Build dated fixtures from today's date unless the test is about a fixed date.
**Tags:** tests, fixtures, spec
**Date:** 2026-10-19

### Local green is not CI green
**Pattern:** The hook suite passed locally and failed in CI three ways. Under `pipefail`, a payload producer's broken-pipe exit leaked into the shim's exit code. The suite used a binary that existed only on the dev machine. A Windows checkout gave CRLF, so a frontmatter parse failed.
**Rule:** Feed test stdin by redirect or process substitution, not a pipe. Build or require each binary a suite needs. Normalise CRLF when a test reads repo text.
**Tags:** tests, ci, hooks, windows
**Date:** 2026-10-22

### Compare a task branch with origin/main before planning
**Pattern:** The Layer task branch `villain/vibe` sat behind `origin/main`. The first review-hardening implementation was built on that stale base, and every unit had to be ported onto the v0.4 tree afterwards.
**Rule:** Before planning on a task branch, run `git fetch` and compare it with `origin/main` (`git rev-list --left-right --count HEAD...origin/main`). Rebase or re-plan before any unit starts, and write the base SHA into the plan.
**Tags:** process, git, orchestration, layer, base
**Date:** 2026-10-24

### Workflow worktree isolation does not branch from your branch
**Pattern:** A Workflow agent with `isolation: 'worktree'` gets a worktree branched from the bare repo's default branch, not from the task branch. An implementer in such a worktree works on the wrong base and nothing warns.
**Rule:** Give each implementer a pinned base SHA. It creates its own worktree with `git worktree add -b <branch> <path> <sha>` and asserts `git rev-parse HEAD` equals that SHA before its first edit.
**Tags:** orchestration, workflow, worktree, git, subagents
**Date:** 2026-10-24

### `validate` reads any feature/number text in a plan as a unit edge
**Pattern:** `vibe-spec validate` (I1) flags every `<feature>/<n>` string in a feature plan as a cross-feature unit dependency, also when it is only a test-ID citation. Under `--strict` the warning fails the tree.
**Rule:** In a feature plan, cite another feature's unit as `<feature> unit-n`, never in the slash form. Keep the slash form for test names and for the plan's own units.
**Tags:** spec, validate, plans, citations
**Date:** 2026-10-24

### A path guard checks the physical path
**Pattern:** `confinePath`, the uninstall deletes and the AGENTS.md writers all judged the spelled path. A symlink in the config, in a parent directory or at the target let a read inject an outside file, a delete leave the target, and a temp + rename replace a link with a regular file.
**Rule:** Resolve the real path of the anchor and of the target (or of its deepest existing ancestor for a new file), then compare with a separator-aware prefix test. Write through an in-repo link, refuse an outside or dangling one, and report each refusal. A lexical check is only a cheap pre-check.
**Tags:** security, symlinks, confinement, install, content
**Date:** 2026-10-24

### Read git status with `-z`, scoped to the project
**Pattern:** The Stop gate read plain `git status --porcelain`. C-quoted names (a space, a non-ASCII byte, a newline, a rename) never matched a file, and toplevel-relative paths missed every change in a project below the git root. A stale receipt passed. A wholly untracked nested project arrives as one `?? sub/` record, so the prefix strip leaves an empty path.
**Rule:** Run `status --porcelain -z -- .`, strip `rev-parse --show-prefix`, consume the origin field of every `R` or `C` record, and treat an empty stripped path as the project directory. In bash, read with `read -r -d ''` from a process substitution.
**Tags:** git, hooks, gate, parsing, nested-project
**Date:** 2026-10-24

### Port fixes onto a moved base after all their tests are RED
**Pattern:** The review-hardening fixes were written for the pre-v0.4 tree. v0.4 added delete and rewrite sites that the old diff never saw: manifest-driven and source-walk uninstall deletes, and the `--only spec` manifest prune that a later review found unguarded.
**Rule:** Port the old tests first and see each one RED on the new base. Apply the old diff only then. After it is green, grep the new base for every delete, `rmdir`, `mv` and rewrite site, partial modes included, and give each site the same guard and a test.
**Tags:** porting, install, uninstall, tdd, review
**Date:** 2026-10-24

### A shim that captures stderr must not need a temp file
**Pattern:** `spec/hooks/run-hook.sh` captured the guard's stderr in a `mktemp` file. With no usable temp directory the capture failed, and a real `blocked:` exit 2 became an allow.
**Rule:** Capture stderr with a descriptor swap into a variable. Test the shim with a probe binary that counts temp files while it runs.
**Tags:** hooks, shim, fail-open, temp-files
**Date:** 2026-10-24

### A differential test must also assert the expected value
**Pattern:** Parity tests compared two writers' output only. The dropped-feature fixture stayed green while both writers carried the bad feature, and no rename record was followed by a second record, so removing the origin step from either gate stayed green.
**Rule:** Besides "both sides agree", assert the absolute expected value, the WARN text, and a precondition floor (for example, that git really reports the fixture). Then delete the guarded line and see the test turn RED.
**Tags:** tests, parity, discriminating-tests, review
**Date:** 2026-10-24

### Scrub every git variable a hook exports before a git fixture runs
**Pattern:** A git hook that runs the engine suite exports `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, and with `git -c` also `GIT_CONFIG_PARAMETERS` or `GIT_CONFIG_COUNT` with `GIT_CONFIG_KEY_n`/`GIT_CONFIG_VALUE_n`. The fixtures then acted on the outer repo, or an injected `status.showUntrackedFiles=all` turned pins red.
**Rule:** The shared test spawn helper deletes all of these variables. A test runs each pin under each injection and checks every deleted name.
**Tags:** tests, git, hermeticity, environment, hooks
**Date:** 2026-10-24

### Pin every ambient input an install or hook test reads
**Pattern:** An uninstall test installed without `--platform`, so host detection read the runner's `PATH` and failed where no `claude` CLI exists. Guard tests read the caller's cursor because `CLAUDE_PROJECT_DIR` was not set to the sandbox.
**Rule:** Pass the platform explicitly and set `CLAUDE_PROJECT_DIR` to the sandbox in each test leg. A test must not depend on the CLIs or the cursor of the machine it runs on.
**Tags:** tests, install, hooks, hermeticity, ci
**Date:** 2026-10-24

### `jq -r` erases JSON types
**Pattern:** `set-state.sh` read the carried feature with `jq -r`. A number or `true` came back as the string `5` or `true`, `false` and the string `"null"` read as no feature, and a trailing newline was trimmed. The engine classified the same values by type, so the two writers disagreed.
**Rule:** Read `jq type` first and classify the value as the other writer does. Use `jq -r` only on a value known to be a string.
**Tags:** jq, bash, parity, cursor
**Date:** 2026-10-24

### `patch` leaves `.orig` files behind
**Pattern:** Two units applied an old hunk with `patch -F3` and committed the `.orig` backup it wrote. Each needed a follow-up commit.
**Rule:** After `patch`, run `git status --porcelain` and delete every `.orig` and `.rej` file before `git add`.
**Tags:** git, patch, porting, hygiene
**Date:** 2026-10-24
### Test scratch is reaped, not leaked
**Pattern:** Every `cargo test` process left one directory per test case in `TMPDIR` and never removed it; a day of worker runs left 14 500 dead directories and 4.3 GiB on a disk with 5.7 GiB free, and the only reclamation was an external 24 h pruner.
**Rule:** Build test scratch through `vibe_core::testscratch::scratch`, which sweeps siblings idle for more than six hours on first use in a process. Never hand-roll a `std::env::temp_dir()` path in a test, and never assume a later process will clean up after this one.
**Tags:** tests, scratch, tmpdir, disk
**Date:** 2026-10-28

### Scratch cleanup requires an explicit owner
**Pattern:** The six-hour sweep recommended by the 2026-10-28 lesson deleted unrelated `vibe-*` data and another live process's scratch. Reusing a computed path deleted collision contents. A scoped owner also leaked its container when a caller had already removed the workspace.
**Rule:** This supersedes the 2026-10-28 six-hour sweep rule. Reserve scratch with exclusive directory creation and retry collisions. Retain the `Scratch` owner through every helper and derived path's last use. On drop, reclaim only the original container with unchanged ownership metadata and marker; an already removed workspace permits removing its owned container. Age, a PID, a name and a marker alone do not prove ownership or inactivity. Leave abandoned or legacy data for separately verified cleanup. CLI callers report reservation errors through the fallible helper; the panic-based shortcut is for tests.
**Tags:** tests, scratch, tmpdir, safety, ownership
**Date:** 2026-11-02

### Fixture adapters retain lifetime owners
**Pattern:** Replacing a `PathBuf` scratch helper with a scope guard left transitive fixture return types, tuple fields and JSON conversions expecting bare paths. A temporary `.join()` discarded the guard before the derived path was used.
**Rule:** Audit the entire fixture return chain when adding cleanup guards. Return the owner alongside derived paths, keep it alive until the last use, and convert only borrowed paths for serialization. Compile all test targets before running suites, then run strict Clippy; express Unix permission modes as octal literals.
**Tags:** tests, rust, ownership, tooling
**Date:** 2026-11-02

### Verify tool availability before invoking matrix commands
**Pattern:** A Homebrew Cargo installation had no `rustup`; a toolchain-list command failed. An unquoted optional glob also failed in zsh before `rg` ran. A rejecting host-CLI shim was still detected as installed, and hooks.sh could not find its hardcoded target/debug binary when Cargo used an isolated build root. A first PATH allowlist omitted grep and basename, producing environment-only failures.
**Rule:** Probe the executable needed by a verification command, rather than inferring it from a related tool. Quote search globs that may match no files. Keep required system utilities in a host-CLI-free allowlisted PATH for optional-tool skip tests, and use an ignored target/debug bridge for suites with a fixed binary path. Report an unavailable toolchain as an unverified matrix leg.
**Tags:** tooling, verification, rust
**Date:** 2026-11-02

### A Windows sharing hold needs data access
**Pattern:** PR66 commit `fe29557` passed macOS checks, but Windows Rust 1.88 CI job `125298715909` renamed the empty workspace while its `access_mode(0)` handle omitted `FILE_SHARE_DELETE`. The metadata-only handle did not protect ownership.
**Rule:** Request directory data-read access with `read(true)` before relying on a Windows no-delete sharing mode. Preserve the original rename-denial regression and add a bare empty-directory hold/release control. If ownership handles are missing, preserve the tree. Use Windows CI on both MSRV and stable as platform evidence; non-Windows green cannot establish Windows sharing semantics.
**Tags:** tests, rust, windows, ownership, safety
**Date:** 2026-11-02


### Compound before reporting merge readiness
**Pattern:** Open PRs retained branch-only design and plan artifacts after their implemented facts already lived in the durable spec. Strict validation passed while the pre-merge drift check failed.
**Rule:** Inventory the whole candidate tree before reporting merge readiness. Promote only missing current facts into the durable spec, remove branch-only artifacts, and run strict validation plus `check-drift --pre-merge`. A green validator alone does not establish compound completion. Check promoted facts against the implementing branch and installation mode.
**Tags:** spec, compound, verification, pre-merge
**Date:** 2026-11-03


### Read verification command requirements
**Pattern:** A batched migration check omitted required `--to docs/spec` and exited 2. An unquoted optional diagnostic glob failed in zsh before the command ran. The sandbox also blocked a process-list probe.
**Rule:** Read the repository’s verification command before batching checks. Preserve its required flags and quote optional search patterns so the tool handles absent matches. Use bounded tool waits and saved check receipts when process inventory is restricted.
**Tags:** tooling, verification, commands
**Date:** 2026-11-03


### Do not infer lock-owner death from age
**Pattern:** A blocking Claude call outlived the freshness window, so a second run stole its lock and the first later overwrote ownership state.
**Rule:** Keep liveness active through blocking work, check the unique owner token before state commit and release, and retain unknown owners when death cannot be established safely. Age and PID alone do not authorize removal; require independently verified manual recovery when portable safe liveness proof is unavailable.
**Tags:** rust, locks, ownership, safety, process
**Date:** 2026-11-02

### Retire registry ownership before releasing the filesystem lock
**Pattern:** Drop unlinked the file before unregistering its token. A real successor registered alongside that retired token and failed its ownership check during an ordinary handoff.
**Rule:** Order registry retirement and filesystem release under the same mutex. Test a real successor acquiring and verifying while its predecessor's Drop is paused at the release boundary; stress alone can miss the race.
**Tags:** rust, locks, concurrency, tests, review
**Date:** 2026-11-02

### Reserve placeholder names across credential owners
**Pattern:** Two imported servers with distinct API_KEY values both became ${API_KEY}; a pre-existing placeholder could also be reused for a new credential.
**Rule:** Reserve placeholders in both input and stored documents and every prior allocation. Give separate credentials distinct names even when their current literals match. When a target cannot translate the allocated name, explicitly report and skip it instead of forwarding the wrong environment variable.
**Tags:** credentials, import, namespaces, translation, tests
**Date:** 2026-11-02

### Make test runtime paths explicit
**Pattern:** A build-note test assumed Cargo's executable path contained /target/, a legacy hook suite required a local target/debug binary despite isolated Cargo output, and direct plugin tests lacked the fixture variables supplied by their wrapper.
**Rule:** Discover test entrypoints before invoking internals, create complete harness fixtures, and supply every runtime path explicitly. Put path-specific assertions under fixture-owned paths, and bridge legacy binary expectations to isolated build output without using global binaries or user configuration.
**Tags:** tests, fixtures, paths, hermeticity, tooling
**Date:** 2026-11-02


### Pin store roots across fixture process boundaries
**Pattern:** Windows CI failed `import_keeps_per_server_env_credentials_distinct` because its injected Unix-flavour Env wrote `.config/vibe`, while the public CLI resolved `%APPDATA%\vibe` and produced no Codex config.
**Rule:** When a test mixes injected paths with a real CLI, pass the fixture's resolved `VIBE_STORE` explicitly. Keep the projection and credential assertions intact; do not change global helpers or skip the host.
**Tags:** tests, fixtures, paths, windows, import
**Date:** 2026-11-02

### Avoid writable executable copies for path-only fixtures
**Pattern:** Ubuntu stable CI failed the build-note fixture at process spawn with `ExecutableFileBusy` / `Text file busy` after copying the binary into its fixture build directory.
**Rule:** A path-warning test can link the already-built executable and put that fixture directory first on PATH when the product preserves matching PATH spelling. Assert the exact registered hook path and warning through the public CLI instead of adding launch retries.
**Tags:** tests, fixtures, executables, paths, ci
**Date:** 2026-11-02

### Preserve CI diagnostic evidence before changing code
**Pattern:** The pinned diagnostics endpoint returned 502 for a failed job; sandbox network access failed, then `gh api` refused terminal escapes. A guessed dated plan path was absent, and a scanner-config search had no matches.
**Rule:** Discover document paths with `rg --files`. Read logs for the exact failed job and head; when the dedicated reader is unavailable, retrieve through the authorized API and preserve escaped log bytes with `--allow-escape-sequences`. Treat a no-match search as evidence, and keep scanner findings distinct from runtime failures.
**Tags:** tooling, ci, diagnostics, discovery, tests
**Date:** 2026-11-02

### Run the pre-merge artifact gate before publication
**Pattern:** Published PRs retained temporary design and plan documents because plain `check-drift` only warned about them.
**Rule:** After promoting durable facts and removing temporary artifacts, run `vibe-spec check-drift --pre-merge` on the final tree before publication. Inventory both `docs/superpowers/` and `.superpowers/`; the drift command scans only the former. A green plain drift check does not prove artifact cleanup.
**Tags:** compound, drift, publication, verification
**Date:** 2026-11-03

<!-- vibe-spec: allow SF15 -->

### Migrate every live cursor reader and writer together
**Pattern:** Rust preferred the new runtime cursor while the legacy setter and JS injection still used old state. Stop read receipts from the old directory. Each implementation passed its own tests while the live surfaces disagreed. The preferred Rust tier also bypassed main hardening retained only in lower tiers. Checking only a cursor and its immediate parent still hid invalid higher ancestors; dangling intermediate links also fooled Rust and JS. Receipt selection repeated the same absence mistake, and basename/lexical normalization selected a different feature receipt.
**Rule:** Define one primary location with an explicit absence-only fallback, preserve legacy bytes, and test legacy-only, new-only, both-present, present-invalid, and no-node controls, using distinct old/new states and absolute expectations for every reader. Port the active producer contract, including NUL Git records, nested roots, rename origins and stat failures, into every preferred tier. Establish absence through the full ancestor chain: require directory type and search permission, reject dangling/looping links, and accept execute-only directories. Apply the same established-absence rule to evidence selection, while keeping receipt metadata validity separate from cursor read permission. Preserve accepted receipt-relative components until filesystem resolution; neither basename truncation nor normalization may hide a missing ancestor. Keep standalone and shared shell classifiers identical with an executable pin. Keep root/layout resolution in its existing primitive owner; run the capability guard before the expensive suite.
**Tags:** flow, migration, cursor, receipts, ownership, parity
**Date:** 2026-11-02

### A block proof covers the complete output and the restricted runtime
**Pattern:** A usage banner containing a block example became a hard block. The first whole-output fix added an awk requirement that the restricted-PATH hook controls did not provide. Shell mv also treated a directory cursor as a destination directory and returned success. Raw newline paths defeated the whole-output grammar, and truthful merged instructions exceeded a binding budget.
**Rule:** Validate the complete block grammar and allowed status, keep the shim within its declared runtime, and escape backslash and C0/DEL bytes in every diagnostic value, and pair real multiline producers with usage/crash and restricted-PATH controls. Assert binding content budgets in executable tests. Refuse a directory leaf before atomic cursor replacement. Update integration fixtures from canonical content rather than pinning an obsolete heading or repo-only configuration.
**Tags:** hooks, shim, fail-safe, cursor, filesystem, tests
**Date:** 2026-11-02

### Keep frozen integrity separate from reviewed source divergence
**Pattern:** A missing live source directory skipped freshness in a partial checkout. Merged hardening also left valid source fixes different from frozen references; re-freezing would erase the behavior comparison.
**Rule:** Skip freshness only for a bare oracle copy. Keep frozen bytes immutable and pin a reviewed divergence with both exact hashes, a known manifest path, and a reason. Reject malformed, duplicate, unknown, missing-source, and stale rows. Report byte-identical sources separately from accepted differences. A source-availability preflight does not make copying transactional.
**Tags:** oracles, freshness, integrity, verification, migration
**Date:** 2026-11-02

### Use read-only Git queries for identity checks
**Pattern:** A final identity check called `git write-tree` only to compare a previously recorded index tree. Git attempted to create `index.lock` and the sandbox refused it, even though the task needed no index mutation.
**Rule:** Compare the index with the recorded tree using `git diff --cached --quiet <tree>`. Use `git rev-parse`, `git ls-files` and source hashes for read-only identity checks. Reserve object-writing commands for an explicitly required mutation; do not escalate a failed write when a read-only query answers the question.
**Tags:** git, verification, sandbox, ownership
**Date:** 2026-11-02

### Stop dependent mutations after a failed check
**Pattern:** A Python staging check raised on inherited main whitespace, but the outer shell continued to `git commit` because the next command had no explicit failure gate. The signed commit contained the approved tree; the required stop did not happen before it.
**Rule:** Run each dependent verification and mutation with explicit exit-code handling. A failed check must stop before commit or push. Classify inherited parent diagnostics separately with read-only comparisons, then continue only after resolving the check's meaning; never rely on a multi-command shell's final exit status to prove every preceding step passed.
**Tags:** git, verification, shell, publication
**Date:** 2026-11-02

### Separate platform-specific filenames from portable protocol controls
**Pattern:** A NUL Git-status regression test created a newline filename alongside portable space, non-ASCII and rename-origin fixtures. Windows rejected the filename with `InvalidFilename` before the scanner assertions ran.
**Rule:** Keep portable filenames and every rename/copy status code active on every platform. Isolate filenames forbidden by Windows in a separate `#[cfg(unix)]` test, and reuse the same behavioral assertions. A native pass does not establish Windows success; retain the actual CI failure and require the repaired Windows CI result.
**Tags:** tests, windows, filenames, hooks, ci
**Date:** 2026-11-02

### Capture CI diagnostics before interpreting them
**Pattern:** An in-progress workflow and job-selection mismatch hid the failing test. A plain HTTP 502 response also failed when interpreted as JSON. Raw job logs exposed the exact Windows filename error.
**Rule:** Select the failing job explicitly. Capture its log through `gh api --allow-escape-sequences repos/<owner>/<repo>/actions/jobs/<job-id>/logs`, retain the raw output, and strip ANSI escapes only in a separate reading copy. Check transport status and response format before parsing JSON; do not treat unavailable in-progress `gh run view` logs as passing evidence.
**Tags:** ci, github, diagnostics, verification
**Date:** 2026-11-02

### Declare the dependency and producer tier of each fixture
**Pattern:** CI physically removed jq and exposed tests that expected validate-state success, check-skills delegates and doctor state reporting despite their declared jq requirements. The restricted PATH also omitted the hook shim's POSIX discovery tools, so `tr: not found` polluted an otherwise correct block. Initial status failures hid later assertions.
**Rule:** Exercise every affected test with jq present and physically absent. Assert each command's named prerequisite failure or degraded result, then retain absolute state and invalid-ancestor controls on readers that support them. Give a shim fixture its declared POSIX tools, assert excluded runtimes are unreachable, mark the selected producer, and compare complete forwarded blocks byte-for-byte. Resolve fixture tools from explicit known paths; inventory files before reading them, and keep patch preconditions atomic so failed lookups or edits cannot leave partial writes.
**Tags:** tests, ci, jq, hooks, fixtures, verification
**Date:** 2026-11-02

### Keep machine cursor fixtures in the writer's format
**Pattern:** A minified hand-written cursor lost its carried feature in the documented no-jq flat-text reader. A directly seeded newline feature also retained its JSON escape in that tier, while jq decoded it; the normal writer rejects that feature. An over-escaped replacement initially asserted the wrong diagnostic bytes.
**Rule:** Build ordinary cursor fixtures through the canonical multiline serialization before mutating one property. For deliberately malformed persisted values, prove the writer rejects them and pin decoded-control and literal-escape diagnostics separately. Assert safe complete grammar, expected block reason and status, and exact producer-to-shim output in each tier; do not expand a historical parser to satisfy an invalid fixture. Use a serializer to construct escaped literals and verify the resulting bytes.
**Tags:** tests, cursor, jq, fixtures, diagnostics
**Date:** 2026-11-02

### Normalize byte arithmetic on the actual supported shell
**Pattern:** Stock Bash 3.2 `printf` returned negative integers for UTF-8 bytes above127. The diagnostic encoder classified those as C0 controls and printed sign-extended hex instead of the original filename. Native checks had selected Homebrew Bash5 through PATH, so they missed the supported-interpreter defect.
**Rule:** Normalize a shell byte to an unsigned octet before classifying control bytes. Preserve non-control UTF-8 bytes exactly and test Latin, multibyte and emoji names beside backslash, tab, newline and DEL. Pin the actual supported interpreter on subprocess PATH and assert its version; invoking a top-level script with that shell does not prove its children use it. Run real producers and exact shim forwarding with jq present and physically absent; keep frozen references unchanged.
**Tags:** bash, portability, utf8, hooks, diagnostics, tests
**Date:** 2026-11-02

### Preserve checker diagnostics on an unexpected result
**Pattern:** The oracle suite reduced a macOS CI failure to rc1/want0 and discarded the actual checker's output. The exact committed source and local stock-shell checks passed, so neither the failed path nor its cause could be recovered from CI.
**Rule:** Capture checker stdout and stderr, and print them when the observed exit differs from the expected exit. Keep successful and expected-negative output unchanged. Prove the reporter with a disposable real mutation, and report an unreproduced CI cause as unresolved; do not waive integrity or change hashes to explain a missing diagnostic.
**Tags:** tests, ci, oracles, diagnostics, verification
**Date:** 2026-11-02

### Isolate inherited project context at the fixture boundary
**Pattern:** Normal, no-jq and spaced CI engine runs passed, but the explicit ambient CLAUDE_PROJECT_DIR leg failed60 tests. Direct fixture writers correctly refused to escape the caller's checkout. The shared test bootstrap scrubbed Git environment but retained the caller's project root; ordinary temporary projects therefore inherited the wrong confinement anchor.
**Rule:** Clear inherited project context at shared fixture bootstrap. Preserve explicit per-case environment injection and production precedence. Add a child-runner regression with an unrelated ambient project and a direct-production control proving that the same explicit root still wins, outside writes fail and bytes remain unchanged. Identify the failing CI step from full logs before attributing the difference to checkout layout; cover normal, ambient, physically no-jq and spaced fixture contexts.
**Tags:** tests, ci, hermeticity, cursor, confinement, environment
**Date:** 2026-11-02

### Finish promotion across the whole retained feature layer
**Pattern:** Completed source retained branch-scoped plans and designs, including inherited phase plans. Exact-topic promotion removed only its bound pair, and index enumeration still listed files already deleted from disk. A dry-run merge command also attempted an object write during a read-only audit. A test entrypoint present on one stack branch was absent on another, and shared wording initially claimed parent hardening that the older branch did not contain.
**Rule:** Promote only missing current contracts, keep later work explicitly pending, and inventory every retained artifact before deletion. Check on-disk existence separately from the index; verify the staged commit tree after staging. Use non-writing Git queries for read-only audits. Inventory commands and verify behavior separately on every stack head before sharing wording. Run strict validation and pre-merge drift after cleanup; never infer compound completion from green source tests.
**Tags:** compound, spec, git, artifacts, verification
**Date:** 2026-11-03

<!-- vibe-spec: allow SF15 -->

### Check stacked integrations at executable boundaries
**Pattern:** A conflict-free base merge introduced a copied binary and shell fixtures outside the shared write/spawn guard, reopening the executable-file-busy window that the stacked test repair closed.
**Rule:** When a base adds executable fixtures, route writers through the shared write_exec/copy_exec helpers and direct test spawns through output/status. Preserve the original behavioral assertions. A clean textual merge does not establish the producer/consumer contract.
**Tags:** rust, tests, merge, concurrency, fixtures
**Date:** 2026-11-02

### Reconcile stacked lessons before regenerating digests
**Pattern:** Independent cleanup commits appended the same lesson after different branch-specific lessons and regenerated different digests, making the stacked PR conflict after publication.
**Rule:** Merge the updated parent before publishing stacked documentation cleanup. Preserve the union of lessons once, regenerate the active-rules digest from that union, and prove the candidate merges with the parent. Matching edits on separate branches do not prove mergeability.
**Tags:** git, merge, compound, lessons, generated, publication
**Date:** 2026-11-03

### Host install tests need an environment allowlist
**Pattern:** Plugin-install tests replaced HOME but inherited host-specific config paths, project settings, and credentials. Real CLIs could write outside the temporary homes.
**Rule:** Launch each host with a clean environment, explicit config/data/cache/state paths, and a temporary working directory. Confirm the host's actual override variables. Prove external paths stay untouched with safe CLI doubles before live install probes.
**Tags:** tests, install, hermeticity, environment, credentials
**Date:** 2026-11-02

### A host cache identity can differ from the manifest version
**Pattern:** Droid registered a local marketplace by its directory basename and cached installed skills under hashed names plus a source revision. Tests assumed the manifest name and version, so a successful installation failed lookup.
**Rule:** Verify each host's registration and cache identity from an isolated real installation. Pin the installed clone's identity and compare real cached files with canonical source. Preserve manifest-version assertions for hosts that use them.
**Tags:** tests, plugins, cache, install, identity
**Date:** 2026-11-02

### Corrected fixtures must fail against the original source
**Pattern:** The first CLI double counted install arguments and failure summaries incorrectly. Its failing output mixed the inherited-config defect with fixture errors.
**Rule:** Derive fixture contracts from actual calls and outputs. After correcting a fixture, run it against the original source again and confirm failure for the intended cause before using its passing result as regression evidence.
**Tags:** tests, fixtures, tdd, review
**Date:** 2026-11-02

### Isolated builds must remain visible to the test harness
**Pattern:** Cargo built successfully in an isolated target directory, but spec and skill tests hardcoded the worktree's target/debug path and reported a missing binary. A temporary target symlink was untracked despite the target/ ignore rule.
**Rule:** Check binary lookup paths before moving build output. If a harness hardcodes target/debug, bridge that path to the isolated build for verification and remove the owned bridge afterward. Verify ignore behavior for symlinks separately from directories.
**Tags:** tests, cargo, isolation, fixtures, gitignore
**Date:** 2026-11-02

### Check generated content and permissions separately
**Pattern:** Canonical active-rules regeneration produced the approved bytes but replaced AGENTS.md with a mode0600 temporary file. A complete source-identity assertion correctly detected its difference from the mode0644 preview.
**Rule:** Check content and filesystem mode separately after atomic generators. Preserve the accepted source-file permissions before publication, then verify both again. Do not change generator behavior during an unrelated repair merely to satisfy its verification fixture.
**Tags:** generators, permissions, verification, git
**Date:** 2026-11-02

### Use authorized network access for remote Git checks
**Pattern:** The sandbox could not resolve github.com during an authorized remote-head check. The same narrowly scoped command succeeded with approved network access and returned the expected PR head.
**Rule:** Distinguish sandbox network admission from repository authentication. Use a scoped escalation for an already authorized remote Git action, verify the exact branch and expected head, and retain the returned SHA before pushing without force.
**Tags:** git, network, sandbox, publication
**Date:** 2026-11-02

### Prepare verification paths before execution
**Pattern:** Exploration opened guessed pinned files and a nonexistent probe script; an early Cargo check also used a target directory outside its admitted write surface. A native fixture exposed physical temporary-path aliases, and a failed preparation was followed by a dependent read.
**Rule:** Discover tracked paths with git ls-tree or rg --files before reading them. Use cataloged skill paths and an explicit admitted CARGO_TARGET_DIR from the first check. Canonicalize disposable roots before exact-path assertions; preserve the asserted file identity. Inspect each prerequisite result before dependent commands. Record setup failures separately from product failures.
**Tags:** verification, tooling, paths, sandbox
**Date:** 2026-11-04

### Validate the budget after merging spec notes
**Pattern:** Combining valid parity notes preserved their contracts but produced a69-line note above the60-line budget.
**Rule:** Run strict validation after joining durable notes. Condense or split the merged concern without dropping executable contracts or newer sections. Parent validation does not validate the combined note.
**Tags:** spec, compound, merge, validation
**Date:** 2026-11-04

### Isolate host capability probes from the first call
**Pattern:** A help probe attempted to open the real user log before the later native probes received isolated configuration paths.
**Rule:** Apply the host environment allowlist, neutral working directory and temporary HOME/config/cache/state paths to help and version commands as well as execution. Match actual help command rows, including qualified and bare spellings. Wait for host plugin activation before inspecting discovery output. Verify native effect separately from server readiness and process exit status.
**Tags:** tests, hosts, isolation, environment
**Date:** 2026-11-04

### Integrate intended deltas across rewritten ancestry
**Pattern:** A branch based on pre-squash history conflicted with current flow code and left an empty old corpus case after its files were discarded.
**Rule:** Preserve the current tree and integrate the branch’s native change when ancestry was rewritten. Retain every requested head as an ancestor, preserve newer contracts and test ownership, and verify the actual generated corpus including obsolete empty directories. Remove only merge-owned leftovers.
**Tags:** git, integration, parity, corpus
**Date:** 2026-11-04

### Keep JSON sentinels distinct from their strings
**Pattern:** Stringifying every non-string feature turned JSON false into a blocking feature-false receipt even though both frozen readers treat false as unnamed.
**Rule:** Test null, false, true, zero and their string spellings independently. Preserve agreed sentinel semantics and document jq, no-jq and JS disagreements explicitly. Prove the failing read path before changing it; never rewrite the frozen oracle to make the port agree.
**Tags:** json, hooks, parity, regression
**Date:** 2026-11-04

### Require unique keys in expected file differences
**Pattern:** A pin listed one changed file twice, matched the observed difference count, and hid a second unlisted changed file.
**Rule:** Reject duplicate expected keys and compare the exact unique expected and observed key sets before matching values. Enforce the invariant at load and matching boundaries, including programmatically constructed inputs. Start from a valid multi-file control and mutate one key to prove the rejection.
**Tags:** parity, validation, metadata, regression
**Date:** 2026-11-04

### Signal only owned live child processes
**Pattern:** Host-probe cleanup attempted to signal process groups for help and API commands that had already exited and been reaped.
**Rule:** Track child lifecycle and signal only live owned children. Keep a live supervisor when descendants can outlive their command, and reclaim the group under that ownership. A stored numeric PID is not ongoing ownership after exit. Test both completed-command exclusion and retained-server timeout cleanup. Use a bounded condition-based descendant-disappearance assertion, retaining a persistent-survivor failure control; signal delivery and OS reaping are asynchronous.
**Tags:** tests, processes, ownership, cleanup
**Date:** 2026-11-04

### Clone-based acceptance needs the intended commit
**Pattern:** The staged installer payload was valid, but clean-clone acceptance checked the previous HEAD and therefore tested missing old packages.
**Rule:** Bind clone-based acceptance to the intended committed candidate or an explicit disposable Git snapshot. Verify the cloned SHA and package bytes before host probes. A working-tree check does not establish what a clean clone installs.
**Tags:** tests, git, snapshots, plugins, verification
**Date:** 2026-11-04

### Audit branch-native behavior after conflict resolution
**Pattern:** The integrated parity pins retained legacy-path evidence, but conflict resolution omitted their source branch's shared migration exemptions. The final migration and literal sweeps failed. A public CLI regression reproduced unwanted rewrites of the pin files. A restoration patch also needed rustfmt, guessed script paths failed during inspection, and a reviewer build used an unapproved target path.
**Rule:** Inventory every branch-native changed file and reconcile its producer and consumer behavior after a conflict. Share exact evidence-path exemptions between migration and the sweep; retain neighboring-file rejection tests. Discover paths before reading them, use the admitted shared build target for review probes, format restored source before checking it, and run each final CI gate against the integrated tree.
**Tags:** integration, parity, migration, verification, discovery
**Date:** 2026-11-04

### Rediscover paths after refreshing a checkout
**Pattern:** Planning inspected a stale checkout, then reused old skill paths after a fast-forward. Reads also assumed a scoped AGENTS.md, Cargo config and a core path module existed. Later scaffold and implementation-preparation passes repeated unchecked optional-path reads despite having discovery output. The resulting missing-file errors obscured which source revision the plan described.
**Rule:** Fetch through authorized access, establish the intended checkout revision, and inventory paths again before reading. Use the canonical source when a compatibility skill path disappeared. Test optional paths before reading them, and keep independent command failures visible instead of hiding them behind a final successful command.
**Tags:** planning, git, discovery, tooling, paths
**Date:** 2026-11-11

### Check plan tasks through the live query
**Pattern:** A readable implementation plan used second-level Task headings, while the spec query recognizes third-level Task headings. Strict root validation passed, but the plan query returned zero tasks.
**Rule:** After authoring a plan, run its feature and plan queries. Verify the intended artifacts, task count and initial statuses. Follow the consuming parser's format instead of treating prose validation as proof of machine-readable structure.
**Tags:** planning, spec, validation, contracts
**Date:** 2026-11-11

### Give remote clients the complete conflict lifecycle
**Pattern:** The spec MCP design could report conflicts and finalize a resolution, but only a filesystem editor could inspect and modify the isolated conflict material. A client restricted to MCP could not finish the workflow.
**Rule:** For every reported conflict, define scoped reads, structured resolution submission and finalization through each supported interface. Bind the resolution workspace and canonical target to separate expected revisions, and test the complete client journey without an external editor.
**Tags:** planning, mcp, conflicts, api, review
**Date:** 2026-11-11

### Recovery owns its writes, not later edits
**Pattern:** Preflight found that the planned crash recovery reused unconditional restore operations, which could overwrite an editor's changes made after interruption. It also found Unicode-only environment capture and inherited external-linter execution in the new API path.
**Rule:** Journal original and transaction-written hashes; restore only transaction-owned bytes and preserve divergent edits as recoverable conflicts. Preserve OS-string path inputs when embedding a CLI. Make external-process capabilities explicit per entry point instead of inheriting legacy opt-ins in a server or transaction validator.
**Tags:** transactions, recovery, cli, mcp, review
**Date:** 2026-11-11

### Verify the compiler selected by the actual command
**Pattern:** An isolated Cargo proxy still selected the system compiler because its toolchain directory was absent from PATH. A later cross-target failure exposed that the successful native run did not prove Rust1.88 compatibility. Dependencies also contained exact packages without declared minimum versions.
**Rule:** Record Cargo and rustc versions in the same environment as verification. Keep native execution, cross-target compilation and declared dependency support separate. Use exact package/version/source exceptions for missing declarations. Preserve earlier receipts with their true identity; a later successful check does not retroactively change them.
**Tags:** rust, toolchains, dependencies, verification
**Date:** 2026-11-12

### Revalidate selected authority after waiting or preflight
**Pattern:** Discovery before locking could outlive a checkout, root or branch change. Repo-first setup then created documents on a switched branch before refusing the operation. A failed managed selection could fall back to legacy code paths, and explicit project selection could retain an unrelated ambient checkout. Branch management also required an ambiguous code checkout for an exact stored branch and could fail while composing its response after publication. Long merge preflight created another interval before publication.
**Rule:** Resolve original caller inputs under the authority lock and recheck them after preflight before writes. Preserve an explicitly captured checkout, reject changed authority and never turn a managed lookup failure into legacy fallback. Reject unsupported selector placement before dispatch or input reads instead of silently using ambient context. Explicit project selection replaces unrelated ambient context. Separate storage-only management from operations requiring code selection, retain root-override checks when code context drops, and validate both source and result authority before publication. Never fabricate a code checkout or head to make a stored spec operation succeed.
**Tags:** context, locking, transactions, identity, review
**Date:** 2026-11-12

### Prove metadata publication and retire only consumed work
**Pattern:** Equal base/planned OIDs could falsely prove a metadata-only snapshot committed. Parsed JSON equality hid byte ownership, resumable setup omitted observations, and a consumed refresh envelope could survive into a later merge. Historical needs-input records also remained reported after another merge included their source.
**Rule:** Journal exact before/after bytes and hashes, validate declared planned hashes, and use explicit commit proof when refs do not move. Complete linked metadata and receipts before journal retirement. Compare all persisted setup fields. Retire an envelope only from exact linked observation/record evidence; recompute outstanding work against the final target while preserving historical records, Pending state and conflict material.
**Tags:** recovery, metadata, history, reconciliation, transactions
**Date:** 2026-11-12

### Keep feature artifacts out of durable completion proof
**Pattern:** Fallback feature documents could count as durable changes, and merge history could lose or invent feature lifecycle evidence. A recorded draft could then be mistaken for a validated completed specification.
**Rule:** Exclude the entire reserved feature-artifact layer, including fallback features and Superpowers documents, from durable promotion proof. Track explicit introduction and completion through all parents. Missing or ambiguous evidence must remain unresolved. Validate a completed tree separately from admitting an ordinary draft write.
**Tags:** spec, promotion, lifecycle, history, validation
**Date:** 2026-11-12

### Bind fixtures and commands to their real environment
**Pattern:** Guessed optional paths, a shell variable named path, inherited Git signing, background maintenance and invalid fixture shapes caused misleading failures. One schema negative used an already-invalid context; an optional document did not trigger the intended validator. A squash fixture also had diverged clone tips, so it exercised ambiguity before squash classification. Earlier successful receipts were easy to confuse with later source.
**Rule:** Discover exact paths before reading, use task-specific variable names and safe literal quoting, and inspect each prerequisite result. Isolate fixture HOME/configuration, disable unrelated Git maintenance and start from valid producer-shaped data. Assert the intended failing field or effect. Preserve failed receipts, source hashes and permissions separately; rerun the affected checks after corrections without modifying frozen oracles or unrelated baselines.
**Tags:** tests, fixtures, tooling, isolation, verification
**Date:** 2026-11-12

### Admit the complete response before publishing effects
**Pattern:** Separate management and refresh reservations each fit while their combined reply did not. Reconciliation saved Pending before accounting for conflicts and validation diagnostics. An immutable invalid configuration could also create a larger error after registration. Permissive MCP schemas initially missed real result fields and variants.
**Rule:** Validate actual producer responses with an independent schema engine and valid-baseline field mutations. Reserve the entire outer response, original ID, escaping, receipts and diagnostics before new effects. Obtain actual merge results in exclusively owned confined scratch when needed, validate predictable configuration first, and revalidate authority afterward. Cleanup must prove filesystem and Git-registration ownership. Keep fresh publication checks and preserve completed proof if actual results diverge.
**Tags:** mcp, schemas, admission, transactions, review
**Date:** 2026-11-12

### Cancellation follows blocking work and publication boundaries
**Pattern:** Synchronous identity discovery occupied async workers, cancelled work could release admission too early, a completion mask covered later refresh work, and resumed setup lacked a cancellation check immediately before its unpublished CAS.
**Rule:** Move blocking discovery off async workers and retain admission through actual execution and cleanup. Check cancellation after waits and immediately before new publication, including retries. After publication, finish its linked bookkeeping with a local completion scope, then restore cancellation for later independent work. Preserve SDK response suppression and expose durable state through scoped reads rather than inventing historical request replay.
**Tags:** async, cancellation, concurrency, recovery, mcp
**Date:** 2026-11-12

### Query status with the command's actual grammar
**Pattern:** A status check copied the feature command's positional argument into the plan command, which requires --feature. Strict validation passed independently; the incorrect query failed before reading any plan.
**Rule:** Check each command's parser or help instead of borrowing a sibling command's argument form. After changing execution status, verify the live task query's rows and status values as well as strict document validation.
**Tags:** cli, planning, contracts, verification
**Date:** 2026-11-12

### Audit prerequisites and paths on each delivered surface
**Pattern:** Final review found an overview claiming no runtime dependency while global spec history invokes Git. A source-relative router link also broke in materialized plugin copies. Verification inspection guessed a nonexistent stop.rs instead of discovering the current hook.rs module.
**Rule:** Inventory actual modules and external executables before naming them. Update overview prerequisites when an implementation adds a runtime tool. Check links in each materialized layout as well as the canonical source; byte-identical copies do not prove their relative paths resolve.
**Tags:** documentation, packaging, discovery, verification
**Date:** 2026-11-12

### Separate preserved CLI bytes from approved additions
**Pattern:** A frozen compatibility test compared aggregate help byte-for-byte after approved commands and selectors were added. It failed on that expected addition before reaching later legacy cases.
**Rule:** Keep exact byte comparisons for unchanged operations. Check approved aggregate additions through an explicit allowlist that preserves legacy entries, descriptions, order and surrounding text. Normalize only the documented formatting difference. Test missing, duplicate and unexpected entries, then run every historical case without rewriting the oracle.
**Tags:** cli, parity, compatibility, tests
**Date:** 2026-11-12

### Build verification copies from the candidate source
**Pattern:** An adapter fixture copied ignored build and review caches and stalled. A clean candidate copy exposed two environment failures: its isolated Node farm contained a mise shim that could not run, and OpenCode could not open its private test server. Native Node and the required local-server permission made the unchanged affected checks pass.
**Rule:** Copy the current tracked and nonignored candidate files with their modes and link targets, excluding unrelated caches. Verify that identity before and after checks. Use native executables inside isolated homes, provide required local test capabilities, preserve failed receipts and rerun affected checks. Distinguish a host test of committed HEAD from a test of the uncommitted candidate.
**Tags:** packaging, fixtures, isolation, verification
**Date:** 2026-11-12

### Recover verification from receipts and observed identities
**Pattern:** A worker credit interruption left a completed test process behind. Restricted process listing, guessed paths and interpreter names, and report parsers that confused qualified test names or omitted a fixture binary caused follow-up probe failures. A restricted advisory refresh also needed explicit network access.
**Rule:** Record child process identities and exit receipts when starting verification. After an interrupted worker, inspect those receipts before rerunning work. Discover actual paths, process arguments and producer schemas; stop and clean only proven owned resources. Keep infrastructure failures and diagnostic mistakes separate from product failures, and preserve the successful retry's exact environment.
**Tags:** tooling, orchestration, recovery, verification
**Date:** 2026-11-12

### Verify publication through authorized network access
**Pattern:** A restricted remote-branch lookup failed with `ssh: Could not resolve hostname github.com: -65563`; the same read-only lookup succeeded with authorized network access and confirmed the branch was absent.
**Rule:** Distinguish a network access boundary from credential failure or remote corruption. Retry the exact read-only lookup with authorized access, preserve its result, and verify the remote branch SHA against the local commit after an authorized push.
**Tags:** git, publication, network, verification
**Date:** 2026-11-12

### Preserve note budgets during promotion
**Pattern:** Replacing temporary artifact links with durable scope and publication status expanded a technology note beyond its 60-line budget; strict validation rejected the 63-line edit and its 61-line revision.
**Rule:** Keep promotion edits within the existing note budget. Consolidate repeated status and links, then rerun strict validation before completing promotion.
**Tags:** spec, promotion, documentation, validation
**Date:** 2026-11-12

### Separate physical authority from Git path operands
**Pattern:** Windows CI exposed verbatim paths rejected by Git, missing directory variables, a relocated Git launcher without its runtime, and embedded templates converted to CRLF.
**Rule:** Keep canonical native paths for confinement and identity; translate only Git path operands without losing native units. Isolate platform directory variables and preserve Git's installed runtime. Pin embedded templates to LF and normalize logical document identifiers separately from physical paths.
**Tags:** windows, paths, confinement, fixtures, portability
**Date:** 2026-11-12

### Retire admitted lock owners before a timed hook exits
**Pattern:** A hook returned success at its work deadline while its detached worker held a project lock. A fixed wall-clock test sometimes expired before acquisition and missed the defect.
**Rule:** Admit managed work atomically against timeout, cancel it through the same control, and join its worker through guard cleanup and publication bookkeeping. Reproduce timeout only after observing the real acquired lock. Test successful rendering with an independent generous work budget; test deadline behavior separately.
**Tags:** hooks, cancellation, locks, deadlines, tests
**Date:** 2026-11-12

### Prove convergence and the complete validation surface
**Pattern:** Live setup exceeded the bounded lock wait, targeted literal checks missed tracked files, and interrupted or guessed diagnostic probes obscured which result was proven.
**Rule:** Accept bounded Busy only with a successful peer, unchanged foreign ownership, and fresh post-completion reads proving one project identity. Run the full tracked sweep and native Windows CI; cross-compilation proves a separate boundary. Retain exact failed receipts, discover paths before reads, quote optional globs, and use task-specific variables before retrying probes.
**Tags:** concurrency, validation, ci, receipts, tooling
**Date:** 2026-11-12


### Rename commands across subprocesses and abbreviated references
**Pattern:** The config namespace migration updated public fixtures but missed a nested subprocess invoking the removed sync command. Full-suite execution caught usage exit 2 where the lock control expected exit 1. Final review also found abbreviated store-command spellings in current spec notes.
**Rule:** Inventory direct argv arrays, helper-generated calls and nested executable scripts before renaming a CLI. Search both full executable spellings and abbreviated command names in current contracts. Run the complete package suite and inspect remaining hits by meaning; keep intentional rejection cases and stored-data literals unchanged.
**Tags:** cli, migration, subprocesses, fixtures, documentation
**Date:** 2026-11-12

### Run plugin tests through their generated fixture contract
**Pattern:** A direct Node invocation omitted the plugin paths that its Rust wrapper generates, so it failed before testing plugin behavior. The fixture-backed command and wrapper passed all 16 tests.
**Rule:** Inspect the test wrapper before invoking an internal runner. Generate both raw and substituted plugin fixtures, set every required environment variable, and retain the exact successful command. Keep fixture setup failures distinct from product failures; never use live user configuration as the fixture.
**Tags:** tests, node, plugins, fixtures, verification
**Date:** 2026-11-12
