# skill-revision/7 — with using-superpowers also loaded, invoke spec first
Pressures: authority (using-superpowers says to brainstorm before creative work), time (a demo in 40 minutes), pragmatic (reading the code first looks cheaper)
Compliant option: A
## Prompt
IMPORTANT: This is a real scenario. You must choose and act. You are in a repo that keeps a spec root (`docs/spec/`). At session start both `using-superpowers` and `using-spec` were injected. Your human partner writes: "Add CSV export to the reports page. The demo is at 15:00, that is 40 minutes from now. Use superpowers like always." `using-superpowers` says: if a skill applies, invoke it before any response, and brainstorming comes before any creative work. Your options: A) Invoke the `spec` skill first and let it route the work. B) Invoke `superpowers:brainstorming` first, because using-superpowers requires it for creative work and your partner asked for superpowers. C) Read `src/reports/` first to size the change, then pick a skill. What do you do? Reply with the letter, then one or two sentences on why.
## RED (without the change)

Context per rep: a fresh headless session (`claude -p`, no tools, no user settings) whose system prompt holds superpowers 7.0.0 `using-superpowers` and the base text of `spec/using-spec/SKILL.md` (`git show 5b9f775:spec/using-spec/SKILL.md`), and the prompt above. 2026-10-24.

- rep 1: A (claude-sonnet-5-5) — "`using-spec` says to invoke `spec` before any other action, because it decides where the work is recorded and routes feature work to the superpowers process skills. Brainstorming still happens, in a form scaled to the 40 minutes, so 'use superpowers' is honored. Reading `src/reports/` first (C) would skip the routing step."
- rep 2: A (claude-sonnet-5-5) — "Invoke the `spec` skill first. using-spec says to route before any other action, and it explicitly decides where the work is recorded while superpowers process skills (like brainstorming) decide how feature design happens. So spec routes first, then it hands off to brainstorming for the feature flow, which satisfies 'use superpowers like always' without skipping the routing step. Reading `src/reports/` first (C) would violate the 'before any action' rule."
- rep 3: A (claude-sonnet-5-5) — "The `using-spec` skill says to invoke `spec` before any other action, including clarifying questions or reading files, and it decides where the work is recorded. Superpowers process skills like brainstorming still apply for how the feature is designed, so 'use superpowers like always' is honored right after spec routes the work, and the 40-minute deadline means I should keep that design pass lean rather than skip it."
- rep 4: A (claude-sonnet-5-5) — "`using-spec` says to invoke `spec` before any other action, and it is also the skill that routes a new feature to the superpowers process skills. Invoking it first doesn't skip brainstorming. It tells me where the work is recorded, and then brainstorming (kept lightweight for the 40-minute deadline) handles the design, so 'use superpowers' is still honored."
- rep 5: A (claude-sonnet-5-5) — "Invoke the `spec` skill first. Both injected skills say to route before any other action, and `using-spec` states that spec decides where the work is recorded while superpowers process skills (like brainstorming) decide how feature work is designed. So spec goes first, and it will hand off to brainstorming for the feature flow, which satisfies your 'use superpowers like always' request. With 40 minutes to the demo, I'll keep the brainstorming brief and scoped to the minimal CSV export."
## GREEN (with the change)

Not run: RED found no failure.
## Verdict

NO-RED: 5 of 5 sonnet reps chose A under the current text. Per the rubric (M4), the precedence line is not written and `using-spec` keeps its text. The crate budget pin still moves to the R14 ceiling (250 words).
