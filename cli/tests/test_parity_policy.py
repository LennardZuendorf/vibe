"""Parity gate: :func:`vibe.policy.decide` equals ``detect-context.sh decide``.

The bash origin's ``decide`` mode does **no** file I/O when a state is passed as
its third argument, so the parity run is hermetic — no sandbox is needed (unlike
the rules-digest parity, which rewrites files). We drive the real
``flow/scripts/detect-context.sh`` through ``bash_ref`` across the full
``state × path`` cross product and assert the Python verdict is byte-identical
(including the trailing newline ``echo`` emits). ``bash_ref`` skips the whole
test when ``bash``/``jq`` is unavailable.

Origin pinned: ``<repo>/flow/scripts/detect-context.sh`` (via ``bash_ref``).
"""

from __future__ import annotations

from vibe import policy

# Paths chosen so every ``case`` arm and both the bare and ``*/``-prefixed globs
# are exercised, plus the leading ``./`` strip and several fall-through allows.
_PATHS = [
    ".agents/skills/vibe/state.json",
    "x/.agents/skills/vibe/state.json",
    ".spec/lessons.md",
    "sub/.spec/lessons.md",
    ".spec/product.md",
    ".spec/tech.md",
    ".spec/design.md",
    ".spec/plan.md",
    "deep/.spec/plan.md",
    ".spec/research.md",
    "CLAUDE.md",
    "AGENTS.md",
    "docs/CLAUDE.md",
    "src/x.py",
    "tests/y.py",
    "pkg/src/a.py",
    "pkg/tests/b.py",
    "src",
    "README.md",
    "docs/guide.md",
    "./src/x.py",
    "./.spec/lessons.md",
    ".//src/x.py",
    "pyproject.toml",
]


def test_decide_matches_bash_origin(bash_ref, machine_data):
    states = list(machine_data["states"])
    mismatches = []
    for state in states:
        for path in _PATHS:
            ref = bash_ref("detect-context.sh", "decide", path, state)
            assert ref.returncode == 0, (path, state, ref.stderr)
            expected = ref.stdout
            got = policy.decide(path, state) + "\n"
            if got != expected:
                mismatches.append(
                    f"state={state!r} path={path!r}: "
                    f"python={got!r} bash={expected!r}"
                )
    assert not mismatches, "policy/bash parity drift:\n" + "\n".join(mismatches)
