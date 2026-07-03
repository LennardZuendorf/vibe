"""Contract tests for ``vibe.provision.settings`` (vibe-cli/9).

The merge writes three fixed hook entries into a target's
``.claude/settings.json``, keyed by event + command. These tests pin the exact
tech.md contract, idempotency, non-clobbering of unrelated user hooks, and the
surgical inverse ``unmerge``.
"""

from __future__ import annotations

import json
import subprocess
import sys

from vibe.provision import settings

# The tech.md data contract, written out literally (not derived from the module
# under test) so this is a genuine byte/value-for-value assertion.
EXPECTED_HOOKS = {
    "UserPromptSubmit": [
        {"hooks": [{"type": "command", "command": "vibe-hook inject", "timeout": 10}]}
    ],
    "PreToolUse": [
        {
            "matcher": "Edit|Write|NotebookEdit",
            "hooks": [{"type": "command", "command": "vibe-hook guard", "timeout": 10}],
        }
    ],
    "Stop": [
        {"hooks": [{"type": "command", "command": "vibe-hook gate", "timeout": 10}]}
    ],
}


def _read(target) -> dict:
    return json.loads(settings.settings_path(target).read_text(encoding="utf-8"))


def test_import_is_stdlib_only():
    # Importing the provisioning leaf must not drag the rich stack in.
    code = (
        "import sys, vibe.provision.settings;"
        "banned=[m for m in ('typer','rich','pydantic') if m in sys.modules];"
        "print(','.join(banned))"
    )
    proc = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    )
    assert proc.stdout.strip() == ""


def test_full_contract_byte_for_byte(target_project):
    changed = settings.merge(target_project)
    assert changed is True

    path = settings.settings_path(target_project)
    assert path.exists()
    data = _read(target_project)

    # The only top-level key on a fresh target is "hooks".
    assert set(data.keys()) == {"hooks"}
    assert data["hooks"] == EXPECTED_HOOKS

    # Spell out each mandated element, so a drift names itself.
    hooks = data["hooks"]
    assert hooks["UserPromptSubmit"][0]["hooks"][0]["command"] == "vibe-hook inject"
    assert hooks["PreToolUse"][0]["matcher"] == "Edit|Write|NotebookEdit"
    assert hooks["PreToolUse"][0]["hooks"][0]["command"] == "vibe-hook guard"
    assert hooks["Stop"][0]["hooks"][0]["command"] == "vibe-hook gate"
    for event in ("UserPromptSubmit", "PreToolUse", "Stop"):
        assert hooks[event][0]["hooks"][0]["type"] == "command"
        assert hooks[event][0]["hooks"][0]["timeout"] == 10


def test_remerge_is_noop(target_project):
    assert settings.merge(target_project) is True
    first = settings.settings_path(target_project).read_text(encoding="utf-8")

    assert settings.merge(target_project) is False
    second = settings.settings_path(target_project).read_text(encoding="utf-8")

    assert first == second
    assert _read(target_project)["hooks"] == EXPECTED_HOOKS


def test_partial_remerge_adds_only_missing(target_project):
    # Only one of the three vibe events is already wired, next to a user hook.
    seed = {
        "hooks": {
            "UserPromptSubmit": [
                {"hooks": [{"type": "command", "command": "vibe-hook inject", "timeout": 10}]}
            ],
            "PreToolUse": [
                {"matcher": "Bash", "hooks": [{"type": "command", "command": "my-linter"}]}
            ],
        }
    }
    path = settings.settings_path(target_project)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(seed, indent=2) + "\n", encoding="utf-8")

    assert settings.merge(target_project) is True
    data = _read(target_project)

    # The already-present inject entry is untouched (no duplicate).
    assert data["hooks"]["UserPromptSubmit"] == EXPECTED_HOOKS["UserPromptSubmit"]
    # The missing guard/gate entries were added.
    guard_group = next(
        g for g in data["hooks"]["PreToolUse"] if g.get("matcher") == "Edit|Write|NotebookEdit"
    )
    assert guard_group["hooks"][0]["command"] == "vibe-hook guard"
    assert data["hooks"]["Stop"] == EXPECTED_HOOKS["Stop"]
    # The user's PreToolUse Bash hook survives beside the added guard group.
    assert {"matcher": "Bash", "hooks": [{"type": "command", "command": "my-linter"}]} in data[
        "hooks"
    ]["PreToolUse"]
    # Now fully wired: another merge is a no-op.
    assert settings.merge(target_project) is False


def test_unrelated_user_hooks_preserved(target_project):
    user_hook_group = {
        "matcher": "Bash",
        "hooks": [{"type": "command", "command": "my-linter", "timeout": 5}],
    }
    seed = {
        "permissions": {"allow": ["Bash(ls:*)"]},
        "model": "sonnet",
        "hooks": {
            "PreToolUse": [user_hook_group],
            "SessionStart": [
                {"hooks": [{"type": "command", "command": "user-session-init"}]}
            ],
        },
    }
    path = settings.settings_path(target_project)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(seed, indent=2) + "\n", encoding="utf-8")

    assert settings.merge(target_project) is True
    data = _read(target_project)

    # Unrelated top-level keys survive untouched.
    assert data["permissions"] == {"allow": ["Bash(ls:*)"]}
    assert data["model"] == "sonnet"

    # The user's PreToolUse Bash hook is preserved alongside vibe's new group.
    pretool = data["hooks"]["PreToolUse"]
    assert user_hook_group in pretool
    assert any(
        g["hooks"][0]["command"] == "vibe-hook guard" for g in pretool
    )
    assert len(pretool) == 2

    # The unrelated SessionStart event is left exactly as the user wrote it.
    assert data["hooks"]["SessionStart"] == seed["hooks"]["SessionStart"]

    # All three vibe events are present.
    assert data["hooks"]["UserPromptSubmit"] == EXPECTED_HOOKS["UserPromptSubmit"]
    assert data["hooks"]["Stop"] == EXPECTED_HOOKS["Stop"]

    # Idempotent even with the user's hooks present.
    assert settings.merge(target_project) is False


def test_unmerge_removes_only_vibe(target_project):
    settings.merge(target_project)
    changed = settings.unmerge(target_project)
    assert changed is True

    data = _read(target_project)
    # A fully-inverted merge leaves no empty "hooks" residue.
    assert "hooks" not in data

    # Nothing left to remove -> no-op.
    assert settings.unmerge(target_project) is False


def test_unmerge_preserves_user_hooks(target_project):
    user_hook_group = {
        "matcher": "Bash",
        "hooks": [{"type": "command", "command": "my-linter", "timeout": 5}],
    }
    seed = {
        "hooks": {
            "PreToolUse": [user_hook_group],
            "SessionStart": [
                {"hooks": [{"type": "command", "command": "user-session-init"}]}
            ],
        },
    }
    path = settings.settings_path(target_project)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(seed, indent=2) + "\n", encoding="utf-8")

    settings.merge(target_project)
    assert settings.unmerge(target_project) is True

    data = _read(target_project)
    # The user's Bash hook survives; vibe's guard group is gone.
    assert data["hooks"]["PreToolUse"] == [user_hook_group]
    assert data["hooks"]["SessionStart"] == seed["hooks"]["SessionStart"]
    # No vibe command lingers anywhere.
    blob = json.dumps(data)
    for command in ("vibe-hook inject", "vibe-hook guard", "vibe-hook gate"):
        assert command not in blob


def test_unmerge_preserves_colocated_command_in_shared_group(target_project):
    # Pathological: a user hand-edits vibe's guard group to also run a linter.
    seed = {
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "Edit|Write|NotebookEdit",
                    "hooks": [
                        {"type": "command", "command": "vibe-hook guard", "timeout": 10},
                        {"type": "command", "command": "my-linter"},
                    ],
                }
            ]
        }
    }
    path = settings.settings_path(target_project)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(seed, indent=2) + "\n", encoding="utf-8")

    assert settings.unmerge(target_project) is True
    data = _read(target_project)
    pretool = data["hooks"]["PreToolUse"]
    assert len(pretool) == 1
    assert pretool[0]["hooks"] == [{"type": "command", "command": "my-linter"}]


def test_helpers_return_contract(target_project):
    assert settings.commands() == {
        "UserPromptSubmit": "vibe-hook inject",
        "PreToolUse": "vibe-hook guard",
        "Stop": "vibe-hook gate",
    }
    entries = settings.hook_entries()
    # Deep copy: mutating the returned mapping must not affect the module.
    entries["UserPromptSubmit"]["hooks"][0]["command"] = "tampered"
    assert settings.commands()["UserPromptSubmit"] == "vibe-hook inject"
    assert settings.settings_path(target_project).name == "settings.json"
    assert settings.settings_path(target_project).parent.name == ".claude"
