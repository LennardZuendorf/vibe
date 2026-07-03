"""Install health report — the ``vibe doctor`` health logic (vibe-cli/13).

Ports ``flow/scripts/doctor.sh`` onto the CLI's Python layer. Runs a fixed set
of read-only checks over a target project and reports each as ``ok`` or ``warn``;
every degraded check carries a concrete, vibe-native fix hint. The report is
warn-only by default (a broken install must still be able to describe itself), so
:func:`run_checks` never mutates and the command exits 0 unless the caller passes
``--exit-code``, which turns any warning into a nonzero status for CI gating.

Checks (mirroring ``doctor.sh``, retargeted at the settings.json adapter):

* **skills** — each bundled skill tree is present under ``<root>/.claude/skills/``.
* **hooks** — each of the three ``vibe-hook`` entries is wired into
  ``<root>/.claude/settings.json`` (the current adapter, NOT the legacy
  ``hooks.json``), keyed by event + command via :mod:`vibe.provision.settings`.
* **cursor** — the flow cursor is absent (idle — normal) or present and valid
  against :mod:`vibe.machine` (ports ``validate-state.sh``).
* **agents_md** — ``AGENTS.md`` (or ``CLAUDE.md``) carries the managed
  ``vibe:instructions`` block, via the shared :mod:`vibe.markers` guard.
* **deps** — each dependency declared in the bundled ``deps.json`` is present on
  disk under ``~/.claude``; an absent dependency degrades (never hard-fails) and
  surfaces its declared ``degrade`` text plus the install fix.

``deps.json`` is read as plain data (name / kind / degrade) — this module does
not depend on the plugins provisioner. Rich-path module: it renders a report and
is never imported on the ``vibe-hook`` hot path.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path

from rich.table import Table
from rich.text import Text

from vibe import cursor, machine, markers
from vibe.provision import settings
from vibe.ui import theme
from vibe.ui.console import console

__all__ = [
    "Check",
    "run_checks",
    "has_failures",
    "render",
    "report",
]

_ASSETS = Path(__file__).resolve().parent / "_assets"
_INSTRUCTIONS_MARKER = "vibe:instructions"
_AGENTS_FILES = ("AGENTS.md", "CLAUDE.md")


@dataclass(frozen=True)
class Check:
    """One health check outcome: ``ok`` or (degraded) ``warn`` with a fix hint."""

    id: str
    ok: bool
    message: str
    fix: str | None = None

    @property
    def status(self) -> str:
        """The literal status word, ``"ok"`` or ``"warn"``."""
        return "ok" if self.ok else "warn"


def _ok(check_id: str, message: str) -> Check:
    return Check(check_id, True, message)


def _warn(check_id: str, message: str, fix: str) -> Check:
    return Check(check_id, False, message, fix)


# --------------------------------------------------------------------------- #
# individual checks
# --------------------------------------------------------------------------- #
def _skill_names() -> list[str]:
    """Bundled skill tree names (``spec``, ``vibe``) — read from the asset dir."""
    root = _ASSETS / "skills"
    if not root.is_dir():
        return []
    return sorted(p.name for p in root.iterdir() if p.is_dir())


def _check_skills(root: Path) -> list[Check]:
    skills_dir = Path(root) / ".claude" / "skills"
    checks: list[Check] = []
    for name in _skill_names():
        dest = skills_dir / name
        if dest.is_dir():
            checks.append(_ok(f"skill.{name}", f"{name} skill installed ({dest})"))
        else:
            checks.append(
                _warn(
                    f"skill.{name}",
                    f"{name} skill not found at {dest}",
                    "run `vibe init` to install the skills into .claude/skills/",
                )
            )
    return checks


def _load_settings(path: Path) -> dict:
    if not path.exists():
        return {}
    text = path.read_text(encoding="utf-8")
    if not text.strip():
        return {}
    data = json.loads(text)
    return data if isinstance(data, dict) else {}


def _hook_command_present(data: dict, event: str, command: str) -> bool:
    hooks_section = data.get("hooks")
    if not isinstance(hooks_section, dict):
        return False
    groups = hooks_section.get(event)
    if not isinstance(groups, list):
        return False
    for group in groups:
        if not isinstance(group, dict):
            continue
        for hook in group.get("hooks", []):
            if isinstance(hook, dict) and hook.get("command") == command:
                return True
    return False


def _check_hooks(root: Path) -> list[Check]:
    path = settings.settings_path(Path(root))
    try:
        data = _load_settings(path)
    except (OSError, ValueError):
        return [
            _warn(
                "hooks",
                f"{path} is present but not valid JSON",
                "fix or delete .claude/settings.json, then run `vibe init`",
            )
        ]
    checks: list[Check] = []
    for event, command in settings.commands().items():
        if _hook_command_present(data, event, command):
            checks.append(_ok(f"hook.{event}", f"{event} -> '{command}' wired"))
        else:
            checks.append(
                _warn(
                    f"hook.{event}",
                    f"{event} hook '{command}' missing from settings.json",
                    "run `vibe init` (or `vibe update`) to wire the settings.json hooks",
                )
            )
    return checks


def _check_cursor(root: Path) -> Check:
    path = cursor.path_for(Path(root))
    fix = (
        "delete .agents/skills/vibe/state.json (or reseed it from the bundled "
        "state.example.json) to reset the cursor to idle"
    )
    if not path.exists():
        return _ok("cursor", "no flow cursor (idle) — normal when not mid-flow")
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return _warn("cursor", f"flow cursor at {path} is not valid JSON", fix)
    if not isinstance(raw, dict):
        return _warn("cursor", f"flow cursor at {path} is not a JSON object", fix)
    for field in ("flow", "phase", "feature", "updated"):
        if field not in raw:
            return _warn("cursor", f"flow cursor missing required field: {field}", fix)
    flow, phase = raw.get("flow"), raw.get("phase")
    if not isinstance(flow, str) or not isinstance(phase, str):
        return _warn("cursor", "flow cursor has a non-string flow/phase", fix)
    m = machine.default()
    key = machine.join_key(flow, phase)
    if key not in m.states:
        return _warn("cursor", f"'{key}' is not a known state in the machine", fix)
    if flow not in m.flows:
        return _warn("cursor", f"flow '{flow}' is not a declared flow", fix)
    if phase not in m.phases:
        return _warn("cursor", f"phase '{phase}' is not a declared phase", fix)
    feature = raw.get("feature") or "none"
    return _ok("cursor", f"flow cursor valid ({key} feature={feature})")


def _check_agents_md(root: Path) -> Check:
    fix = "run `vibe init` (or `vibe update`) to merge the vibe:instructions block"
    for name in _AGENTS_FILES:
        path = Path(root) / name
        if not path.is_file():
            continue
        try:
            text = path.read_text(encoding="utf-8")
            present = markers.has_region(text, _INSTRUCTIONS_MARKER)
        except markers.MarkerError as exc:
            return _warn(
                "agents_md",
                f"{name} has reversed vibe:instructions markers ({exc})",
                "fix the marker pair in AGENTS.md by hand, then run `vibe update`",
            )
        except OSError:
            continue
        if present:
            return _ok("agents_md", f"{name} carries the vibe:instructions block")
        return _warn(
            "agents_md",
            f"{name} has no managed vibe:instructions block",
            fix,
        )
    joined = " / ".join(_AGENTS_FILES)
    return _warn("agents_md", f"no {joined} found in {root}", fix)


def _load_deps() -> list[dict]:
    text = (_ASSETS / "deps.json").read_text(encoding="utf-8")
    data = json.loads(text)
    return [d for d in data.get("deps", []) if isinstance(d, dict)]


def _find_named(root: Path, name: str, max_depth: int = 5) -> bool:
    target = name.lower()
    stack: list[tuple[Path, int]] = [(root, 1)]
    while stack:
        directory, depth = stack.pop()
        if depth > max_depth:
            continue
        try:
            entries = list(directory.iterdir())
        except OSError:
            continue
        for entry in entries:
            if entry.name.lower() == target:
                return True
            if entry.is_dir():
                stack.append((entry, depth + 1))
    return False


def _dep_present(name: str) -> bool:
    home = Path.home()
    if (home / ".claude" / "skills" / name).is_dir():
        return True
    plugins_dir = home / ".claude" / "plugins"
    if not plugins_dir.is_dir():
        return False
    return _find_named(plugins_dir, name)


_GIT_SOURCE = re.compile(r"^([\w.-]+/[\w.-]+|(?:https?|git|ssh)://\S+|git@\S+)$")


def _dep_installable(source: str) -> bool:
    """True when a dep ``source`` yields an installable git marketplace.

    Mirrors ``provision.plugins`` installability so the fix hint never tells the
    user to ``vibe plugins install`` a dep that has no auto-install source.
    """
    return bool(_GIT_SOURCE.match((source or "").strip()))


def _check_deps() -> list[Check]:
    checks: list[Check] = []
    for dep in _load_deps():
        name = dep.get("name", "")
        if not name:
            continue
        kind = dep.get("kind", "")
        degrade = dep.get("degrade", "")
        if _dep_present(name):
            checks.append(_ok(f"dep.{name}", f"{kind} '{name}' present on disk"))
        else:
            if _dep_installable(dep.get("source", "")):
                fix = f"run `vibe plugins install` to install {name}"
            else:
                fix = (
                    f"{name} has no auto-install source in deps.json — install it "
                    "manually (see its deps.json source)"
                )
            checks.append(
                _warn(f"dep.{name}", f"{kind} '{name}' not found — degrade: {degrade}", fix)
            )
    return checks


# --------------------------------------------------------------------------- #
# aggregation + rendering
# --------------------------------------------------------------------------- #
def run_checks(root: Path) -> list[Check]:
    """Run every health check against ``root`` and return the outcomes in order.

    Read-only: no filesystem writes, no cursor mutation. Degrades gracefully — a
    malformed settings.json / cursor / AGENTS.md becomes a ``warn``, never an
    exception.
    """
    root = Path(root)
    checks: list[Check] = []
    checks.extend(_check_skills(root))
    checks.extend(_check_hooks(root))
    checks.append(_check_cursor(root))
    checks.append(_check_agents_md(root))
    checks.extend(_check_deps())
    return checks


def has_failures(checks: list[Check]) -> bool:
    """True when any check degraded (is not ``ok``) — the ``--exit-code`` gate."""
    return any(not check.ok for check in checks)


def render(root: Path, checks: list[Check]) -> None:
    """Render the health report as a rich table plus per-warning fix hints."""
    console.print(f"vibe doctor — {root}", style=theme.get_heading_style(), highlight=False)

    table = Table(box=None, pad_edge=False, show_header=True, header_style=theme.get_label_style())
    table.add_column("", no_wrap=True)
    table.add_column("check", no_wrap=True, style=theme.get_accent_style())
    table.add_column("detail", overflow="fold", style=theme.get_value_style())
    for check in checks:
        if check.ok:
            badge = Text("ok  ", style=theme.get_success_style())
        else:
            badge = Text("warn", style=theme.get_warning_style())
        table.add_row(badge, check.id, Text(check.message))
    console.print(table)

    warnings = [c for c in checks if not c.ok]
    if warnings:
        console.print("Fixes:", style=theme.get_heading_style(), highlight=False)
        for check in warnings:
            line = Text()
            line.append(f"  {check.id}: ", style=theme.get_accent_style())
            line.append(check.fix or "", style=theme.get_dim_style())
            console.print(line, soft_wrap=True)
        console.print(
            f"{len(warnings)} of {len(checks)} checks degraded",
            style=theme.get_warning_style(),
            highlight=False,
        )
    else:
        console.print(
            f"all {len(checks)} checks ok",
            style=theme.get_success_style(),
            highlight=False,
        )


def report(root: Path) -> list[Check]:
    """Run the checks and render the report; return the checks for the caller.

    The command layer uses the returned list to decide the exit status (see
    :func:`has_failures`) — this keeps :func:`run_checks` side-effect free while
    a single call both prints and yields the data behind ``--exit-code``.
    """
    checks = run_checks(root)
    render(root, checks)
    return checks
