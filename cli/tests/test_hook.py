"""Unit 7 — ``vibe-hook`` inject|guard|gate + self-location.

In-process ``main([...])`` is preferred (env/cwd/stdin controlled via
``monkeypatch``, exit code is the return value, stderr via ``capsys``); a real
subprocess is reserved only for the symlink path-parity assertion. Every
self-location test controls ``CLAUDE_PROJECT_DIR`` explicitly — the suite runs
inside Claude Code where it is otherwise set in the ambient environment.
"""

from __future__ import annotations

import io
import json
import subprocess
import sys
from pathlib import Path

import pytest

from vibe import cursor, hook, orders

_STATE_JSON_REL = ".agents/skills/vibe/state.json"


def _guard_stdin(path: str) -> str:
    return json.dumps({"tool_input": {"file_path": path}})


def _set_project(monkeypatch, root: Path) -> None:
    monkeypatch.setenv("CLAUDE_PROJECT_DIR", str(root))


# ── guard: exit 2 on a hard block ────────────────────────────────────────────
def test_guard_hard_block_exits_2(monkeypatch, capsys, target_project):
    """A direct state.json edit is a state-independent hard block: exit 2."""
    _set_project(monkeypatch, target_project)
    monkeypatch.setattr(sys, "stdin", io.StringIO(_guard_stdin(_STATE_JSON_REL)))

    rc = hook.main(["guard"])

    assert rc == 2
    err = capsys.readouterr().err
    assert "vibe-guard: BLOCKED" in err
    assert "set-state.sh, never by direct edit" in err


def test_guard_hard_block_absolute_path(monkeypatch, capsys, target_project):
    """An absolute path under the project root is relativized, still blocks."""
    _set_project(monkeypatch, target_project)
    abs_path = str(target_project / _STATE_JSON_REL)
    monkeypatch.setattr(sys, "stdin", io.StringIO(_guard_stdin(abs_path)))

    assert hook.main(["guard"]) == 2
    assert "BLOCKED" in capsys.readouterr().err


def test_guard_notebook_path_blocks(monkeypatch, capsys, target_project):
    """NotebookEdit carries tool_input.notebook_path, still routed to policy."""
    _set_project(monkeypatch, target_project)
    payload = json.dumps({"tool_input": {"notebook_path": _STATE_JSON_REL}})
    monkeypatch.setattr(sys, "stdin", io.StringIO(payload))

    assert hook.main(["guard"]) == 2


# ── guard: allow / degrade paths ─────────────────────────────────────────────
def test_guard_allow_is_silent_exit_0(monkeypatch, capsys, target_project):
    """An ordinary path at idle is allowed: silent exit 0."""
    _set_project(monkeypatch, target_project)
    monkeypatch.setattr(sys, "stdin", io.StringIO(_guard_stdin("README.md")))

    assert hook.main(["guard"]) == 0
    out = capsys.readouterr()
    assert out.err == ""
    assert out.out == ""


def test_guard_empty_stdin_exits_0(monkeypatch, target_project):
    """Empty / unparseable stdin degrades to exit 0 (never break a session)."""
    _set_project(monkeypatch, target_project)
    monkeypatch.setattr(sys, "stdin", io.StringIO(""))
    assert hook.main(["guard"]) == 0

    monkeypatch.setattr(sys, "stdin", io.StringIO("not json {"))
    assert hook.main(["guard"]) == 0


def test_guard_warn_verdict_exit_0(monkeypatch, capsys, target_project):
    """A warn verdict (AGENTS.md) prints to stderr but exits 0."""
    _set_project(monkeypatch, target_project)
    monkeypatch.setattr(sys, "stdin", io.StringIO(_guard_stdin("AGENTS.md")))

    assert hook.main(["guard"]) == 0
    assert "vibe-guard: warn" in capsys.readouterr().err


# ── inject: emits orders ─────────────────────────────────────────────────────
def test_inject_emits_orders_for_cursor_state(monkeypatch, capsys, target_project):
    """inject prints the orders block for the cursor state, feature-interpolated."""
    _set_project(monkeypatch, target_project)
    cursor.write(cursor.path_for(target_project), "feature.impl", feature="widgets")

    assert hook.main(["inject"]) == 0
    out = capsys.readouterr().out
    expected = orders.resolve("feature.impl", "widgets")
    assert out.strip() == expected.strip()
    assert "widgets" in out
    assert "<feature>" not in out


def test_inject_idle_when_no_cursor(monkeypatch, capsys, target_project):
    """A bare target (no cursor) injects the idle orders via the bundled skill."""
    _set_project(monkeypatch, target_project)

    assert hook.main(["inject"]) == 0
    assert capsys.readouterr().out.strip() == orders.resolve("idle").strip()


def test_inject_prefers_target_claude_skills(monkeypatch, capsys, target_project):
    """A provisioned .claude/skills/vibe/SKILL.md block wins over the bundle."""
    _set_project(monkeypatch, target_project)
    cursor.write(cursor.path_for(target_project), "feature.impl", feature="gizmo")
    skill_md = target_project / ".claude" / "skills" / "vibe" / "SKILL.md"
    skill_md.parent.mkdir(parents=True, exist_ok=True)
    skill_md.write_text(
        "<!-- vibe:orders:feature.impl -->\n"
        "CUSTOM ORDERS for <feature>\n"
        "<!-- /vibe:orders -->\n",
        encoding="utf-8",
    )

    assert hook.main(["inject"]) == 0
    assert capsys.readouterr().out.strip() == "CUSTOM ORDERS for gizmo"


# ── gate: warn-only, always exit 0 ───────────────────────────────────────────
def test_gate_idle_silent_exit_0(monkeypatch, capsys, target_project):
    """At idle the gate emits nothing and exits 0."""
    _set_project(monkeypatch, target_project)

    assert hook.main(["gate"]) == 0
    assert capsys.readouterr().err == ""


def test_gate_non_idle_warns_and_exits_0(monkeypatch, capsys, target_project):
    """A non-idle state warns (advance nudge with next states) but exits 0."""
    _set_project(monkeypatch, target_project)
    cursor.write(cursor.path_for(target_project), "feature.verify")

    assert hook.main(["gate"]) == 0
    err = capsys.readouterr().err
    assert "vibe-gate:" in err
    assert "next: feature.compound" in err


def test_gate_impl_state_no_git_repo_is_safe(monkeypatch, capsys, target_project):
    """feature.impl on a non-git target skips the git smell without crashing."""
    _set_project(monkeypatch, target_project)
    cursor.write(cursor.path_for(target_project), "feature.impl", feature="foo")

    assert hook.main(["gate"]) == 0


# ── self-location: env set vs marker walk ────────────────────────────────────
def test_find_project_root_uses_env(monkeypatch, tmp_path):
    """CLAUDE_PROJECT_DIR wins verbatim over any marker walk."""
    monkeypatch.setenv("CLAUDE_PROJECT_DIR", str(tmp_path))
    assert hook.find_project_root(start=tmp_path / "nowhere") == tmp_path


def test_find_project_root_marker_walk(monkeypatch, tmp_path):
    """With env unset, the upward .spec/.git marker walk finds the root."""
    monkeypatch.delenv("CLAUDE_PROJECT_DIR", raising=False)
    root = tmp_path / "proj"
    (root / ".spec").mkdir(parents=True)
    nested = root / "a" / "b" / "c"
    nested.mkdir(parents=True)

    assert hook.find_project_root(start=nested) == root


def test_find_project_root_marker_walk_git(monkeypatch, tmp_path):
    """A .git entry (file or dir) is a valid marker too."""
    monkeypatch.delenv("CLAUDE_PROJECT_DIR", raising=False)
    root = tmp_path / "proj"
    nested = root / "x" / "y"
    nested.mkdir(parents=True)
    (root / ".git").write_text("gitdir: elsewhere\n", encoding="utf-8")

    assert hook.find_project_root(start=nested) == root


def test_guard_marker_walk_from_cwd_blocks(monkeypatch, capsys, tmp_path):
    """With env unset, guard self-locates from CWD and still blocks state.json."""
    monkeypatch.delenv("CLAUDE_PROJECT_DIR", raising=False)
    root = tmp_path / "proj"
    (root / ".spec").mkdir(parents=True)
    monkeypatch.chdir(root)
    monkeypatch.setattr(sys, "stdin", io.StringIO(_guard_stdin(_STATE_JSON_REL)))

    assert hook.main(["guard"]) == 2
    assert "BLOCKED" in capsys.readouterr().err


# ── path-parity: real path vs symlinked invocation ───────────────────────────
def test_guard_verdict_symlink_path_parity(tmp_path):
    """Invocation-path parity: byte-identical verdict via both entry points.

    Self-location is env/CWD-based and package assets resolve via each module's
    own ``__file__`` — neither depends on ``sys.argv[0]`` — so symlinking the
    console script must not change the verdict. (The discriminating "search
    markers, don't count hops" pin is :func:`test_find_project_root_marker_walk`,
    which asserts a controlled ``start`` walks up to the marked root.)
    """
    real = Path(sys.executable).parent / "vibe-hook"
    if not real.exists():
        pytest.skip("vibe-hook console script not found next to the interpreter")

    project = tmp_path / "proj"
    project.mkdir()
    link = tmp_path / "linked-vibe-hook"
    link.symlink_to(real)

    env = {
        "PATH": str(Path(sys.executable).parent) + ":/usr/bin:/bin",
        "CLAUDE_PROJECT_DIR": str(project),
    }
    stdin = _guard_stdin(_STATE_JSON_REL)

    def run(binary: Path) -> tuple[int, str, str]:
        proc = subprocess.run(
            [str(binary), "guard"],
            input=stdin,
            capture_output=True,
            text=True,
            env=env,
        )
        return proc.returncode, proc.stdout, proc.stderr

    real_result = run(real)
    link_result = run(link)

    assert real_result[0] == 2
    assert real_result == link_result
    assert "BLOCKED" in real_result[2]
