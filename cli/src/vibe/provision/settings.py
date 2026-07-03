"""Idempotent merge of the vibe hook entries into ``.claude/settings.json``.

``vibe init`` wires three per-turn hooks into the target project's Claude Code
``settings.json``. Unlike the plugin's ``hooks.json`` (which runs shell shims
through ``${CLAUDE_PLUGIN_ROOT}``), settings.json hooks invoke the bare
``vibe-hook`` binary on ``PATH`` and self-locate the project, so the commands
are literal and fixed:

===================  ===============================  ================
event                command                          matcher
===================  ===============================  ================
UserPromptSubmit     ``vibe-hook inject``             (none)
PreToolUse           ``vibe-hook guard``              Edit|Write|NotebookEdit
Stop                 ``vibe-hook gate``               (none)
===================  ===============================  ================

Every entry carries ``timeout: 10``. The merge is keyed by *event + command*:
it adds only entries that are missing, so re-running is a no-op and unrelated
user hooks are never rewritten. :func:`unmerge` is the exact inverse — it drops
only vibe's commands, preserving co-located user hooks and empty-pruning what it
emptied.

Stdlib-only (``json``/``os``) — no typer, rich, or pydantic.
"""

from __future__ import annotations

import copy
import json
import os
import tempfile
from pathlib import Path

__all__ = [
    "settings_path",
    "hook_entries",
    "commands",
    "merge",
    "unmerge",
]

_TIMEOUT = 10

_HOOK_ENTRIES: dict[str, dict] = {
    "UserPromptSubmit": {
        "hooks": [{"type": "command", "command": "vibe-hook inject", "timeout": _TIMEOUT}],
    },
    "PreToolUse": {
        "matcher": "Edit|Write|NotebookEdit",
        "hooks": [{"type": "command", "command": "vibe-hook guard", "timeout": _TIMEOUT}],
    },
    "Stop": {
        "hooks": [{"type": "command", "command": "vibe-hook gate", "timeout": _TIMEOUT}],
    },
}

_COMMANDS: dict[str, str] = {
    event: entry["hooks"][0]["command"] for event, entry in _HOOK_ENTRIES.items()
}


def settings_path(target: Path) -> Path:
    """Return the ``.claude/settings.json`` path under *target*."""
    return Path(target) / ".claude" / "settings.json"


def hook_entries() -> dict[str, dict]:
    """Return a deep copy of the canonical event → hook-group mapping."""
    return copy.deepcopy(_HOOK_ENTRIES)


def commands() -> dict[str, str]:
    """Return the canonical event → command string mapping."""
    return dict(_COMMANDS)


def _load(path: Path) -> dict:
    if not path.exists():
        return {}
    text = path.read_text(encoding="utf-8")
    if not text.strip():
        return {}
    data = json.loads(text)
    if not isinstance(data, dict):
        raise ValueError(f"settings.json is not a JSON object: {path}")
    return data


def _write(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(data, indent=2) + "\n"
    fd, tmp = tempfile.mkstemp(dir=str(path.parent), prefix=path.name + ".", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(payload)
        os.replace(tmp, path)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def _has_command(groups: list, command: str) -> bool:
    for group in groups:
        if not isinstance(group, dict):
            continue
        for hook in group.get("hooks", []):
            if isinstance(hook, dict) and hook.get("command") == command:
                return True
    return False


def merge(target: Path) -> bool:
    """Ensure the three vibe hook entries exist in *target*'s settings.json.

    Adds only the entries whose command is absent (keyed by event + command),
    leaving every unrelated user hook untouched. Returns ``True`` when the file
    was changed, ``False`` when it was already fully wired (idempotent no-op).
    """
    path = settings_path(target)
    data = _load(path)
    hooks_section = data.get("hooks")
    if not isinstance(hooks_section, dict):
        hooks_section = {}

    changed = False
    for event, entry in _HOOK_ENTRIES.items():
        command = _COMMANDS[event]
        groups = hooks_section.get(event)
        if not isinstance(groups, list):
            groups = []
        if not _has_command(groups, command):
            groups.append(copy.deepcopy(entry))
            changed = True
        hooks_section[event] = groups

    if not changed:
        return False

    data["hooks"] = hooks_section
    _write(path, data)
    return True


def unmerge(target: Path) -> bool:
    """Remove only vibe's hook entries from *target*'s settings.json.

    Per-command inverse of :func:`merge`: drops each ``vibe-hook`` command from
    its event, keeping any co-located user hooks; prunes a hook-group emptied of
    all its hooks, an event list emptied of all its groups, and an emptied
    top-level ``hooks`` key. Returns ``True`` when the file was changed.
    """
    path = settings_path(target)
    if not path.exists():
        return False
    data = _load(path)
    hooks_section = data.get("hooks")
    if not isinstance(hooks_section, dict):
        return False

    changed = False
    for event, command in _COMMANDS.items():
        groups = hooks_section.get(event)
        if not isinstance(groups, list):
            continue
        surviving_groups = []
        for group in groups:
            if not isinstance(group, dict):
                surviving_groups.append(group)
                continue
            hooks = group.get("hooks", [])
            kept = [
                hook
                for hook in hooks
                if not (isinstance(hook, dict) and hook.get("command") == command)
            ]
            if len(kept) != len(hooks):
                changed = True
            if not kept:
                continue
            group["hooks"] = kept
            surviving_groups.append(group)
        if surviving_groups:
            hooks_section[event] = surviving_groups
        else:
            hooks_section.pop(event, None)

    if not changed:
        return False

    if hooks_section:
        data["hooks"] = hooks_section
    else:
        data.pop("hooks", None)
    _write(path, data)
    return True
