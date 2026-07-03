"""``vibe init [PATH]`` — one-command project provisioning (vibe-cli/11).

Orchestration ONLY. Each step delegates to a tested leaf and this module just
sequences them and reports:

1. PATH prereq check — warn (never silently) when ``vibe`` / ``vibe-hook`` are
   not resolvable on ``PATH``, so the settings.json hooks reference binaries that
   would vanish and silently no-op (the ``uvx`` ephemeral-install trap).
2. copy the bundled skill trees (:mod:`vibe.provision.skills`).
3. merge the three per-turn hook entries (:mod:`vibe.provision.settings`).
4. merge the managed ``AGENTS.md`` block (:mod:`vibe.provision.agents_md`).
5. seed the flow cursor (:mod:`vibe.cursor`) and gitignore it.
6. regenerate the active-rules digest (:mod:`vibe.rules`).
7. offer the Claude Code plugin dependencies (:mod:`vibe.provision.plugins`).

``--only spec`` installs the standalone spec framework alone (spec tree only; no
flow wiring). ``--dry-run`` prints the plan and performs ZERO filesystem writes —
the leaves that lack a native ``dry_run`` parameter are simply gated on it here.
``--yes`` is accepted for parity; ``init`` is additive, idempotent and never
prompts, so it has no interactive gate to bypass.

Exposes the ``register(app)`` wiring contract so ``app.py`` (unit 16) mounts the
command without this module importing ``app.py``.
"""

from __future__ import annotations

import shutil
from pathlib import Path

import typer
from rich.panel import Panel
from rich.table import Table

import vibe
from vibe import cursor, rules
from vibe.errors import VibeError
from vibe.markers import MarkerError
from vibe.provision import agents_md, plugins, settings, skills
from vibe.ui import theme
from vibe.ui.console import console

_BINARIES = ("vibe", "vibe-hook")
_GITIGNORE_PATTERN = cursor.RELPATH.as_posix()
_GITIGNORE_COMMENT = "# vibe mutable flow cursor (runtime; do not commit)"


def _binary_on_path(name: str) -> str | None:
    """Absolute path of a console-script on ``PATH``, or ``None`` if absent.

    A dedicated indirection (not an inline ``shutil.which``) so the ephemeral
    prereq check is monkeypatchable without perturbing ``plugins``' own
    ``shutil.which`` use for the ``claude`` binary.
    """
    return shutil.which(name)


def _spec_assets_dir() -> Path:
    """Absolute path of the bundled spec skill tree (located via the package)."""
    return Path(vibe.__file__).resolve().parent / "_assets" / "skills" / "spec"


def _warn(message: str) -> None:
    console.print(f"[{theme.get_warning_style()}]init:[/] {message}", highlight=False)


def _ensure_gitignore(root: Path) -> bool:
    """Append the cursor pattern to ``root/.gitignore`` unless already ignored."""
    gitignore = root / ".gitignore"
    if gitignore.exists() and _GITIGNORE_PATTERN in gitignore.read_text(encoding="utf-8"):
        return False
    with gitignore.open("a", encoding="utf-8") as handle:
        handle.write(f"\n{_GITIGNORE_COMMENT}\n{_GITIGNORE_PATTERN}\n")
    return True


def _render_summary(rows: list[tuple[str, str]], *, dry_run: bool) -> None:
    grid = Table.grid(padding=(0, 2))
    grid.add_column(justify="right", style=theme.get_label_style(), no_wrap=True)
    grid.add_column(style=theme.get_value_style())
    for label, detail in rows:
        grid.add_row(label, detail)
    title = "vibe init — dry-run plan" if dry_run else "vibe init — provisioned"
    console.print(
        Panel(
            grid,
            title=title,
            title_align="left",
            border_style=theme.get_card_border_style(),
        )
    )


def init_command(
    path: Path = typer.Argument(
        None,
        help="Project root to provision (default: current directory).",
        show_default=False,
    ),
    yes: bool = typer.Option(
        False, "--yes", "-y", help="Assume yes (reserved; init never prompts)."
    ),
    only: str = typer.Option(
        None, "--only", help="Provision a single half. Only 'spec' is supported.",
        show_default=False,
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Print the plan and write nothing."
    ),
) -> None:
    """Provision the vibe flow harness into a target project."""
    root = (path or Path.cwd()).resolve()

    if only is not None and only != "spec":
        console.print(
            f"[{theme.get_error_style()}]init:[/] --only accepts only 'spec' "
            f"(got {only!r}).",
            highlight=False,
        )
        raise typer.Exit(2)
    spec_only = only == "spec"

    prefix = "[dry-run] " if dry_run else ""
    console.print(
        f"[{theme.get_heading_style()}]{prefix}vibe init[/] "
        f"[{theme.get_dim_style()}]{root}[/]",
        highlight=False,
    )

    summary: list[tuple[str, str]] = []

    # 1. PATH prereq — the hooks invoke the bare binaries; warn loudly if they
    #    are only ephemeral (otherwise the settings.json hooks silently no-op).
    if not spec_only:
        missing = [name for name in _BINARIES if _binary_on_path(name) is None]
        if missing:
            _warn(
                f"{' and '.join(missing)} not resolvable on PATH — the "
                "settings.json hooks would silently no-op (ephemeral install?). "
                "Install persistently, e.g. `uv tool install vibe-flow`, then "
                "re-run."
            )

    # 2. skills — full copies both trees; spec-only copies just the spec tree.
    if spec_only:
        if not dry_run:
            shutil.copytree(
                _spec_assets_dir(),
                root / ".claude" / "skills" / "spec",
                dirs_exist_ok=True,
            )
        summary.append(("skills", "spec"))
        _render_summary(summary, dry_run=dry_run)
        return

    skills.install(root, dry_run=dry_run)
    summary.append(("skills", "spec, vibe"))

    # 3. settings.json hook entries.
    if not dry_run:
        try:
            settings.merge(root)
        except ValueError as exc:
            _warn(f"settings.json left intact (unparseable JSON): {exc}")
    summary.append(("settings.json", ", ".join(settings.commands().values())))

    # 4. AGENTS.md managed block (must precede the active-rules digest so the
    #    file exists for step 6 to write into).
    if not dry_run:
        try:
            merged = agents_md.merge(root)
            if merged.warning:
                _warn(merged.warning)
        except (VibeError, MarkerError) as exc:
            _warn(f"AGENTS.md not merged: {exc}")
    summary.append(("AGENTS.md", "vibe:instructions block"))

    # 5. seed the cursor and gitignore it.
    if not dry_run:
        cursor.seed(cursor.path_for(root))
        _ensure_gitignore(root)
    summary.append(("cursor", f"seeded + gitignored ({_GITIGNORE_PATTERN})"))

    # 6. active-rules digest.
    if not dry_run:
        try:
            rules.write_rules(root)
        except MarkerError as exc:
            _warn(f"active-rules not regenerated: {exc}")
    summary.append(("active-rules", "regenerated"))

    # 7. install the plugin dependencies (self-degrades when claude is absent).
    presult = plugins.install_deps(dry_run=dry_run)
    if dry_run:
        plugin_detail = "planned (dry-run)"
    elif not presult.claude_present:
        plugin_detail = "manual — claude not on PATH (see above)"
    elif presult.failed:
        ok = len(presult.ran) - len(presult.failed)
        plugin_detail = f"{ok}/{len(presult.ran)} commands ok (see above)"
    else:
        plugin_detail = "marketplaces added + installed"
    summary.append(("plugins", plugin_detail))

    _render_summary(summary, dry_run=dry_run)
    if not dry_run:
        console.print(
            f"[{theme.get_dim_style()}]next:[/] run [b]vibe status[/], then "
            "[b]vibe go feature.design --feature <name>[/] to start a feature "
            "(or [b]vibe go quick.triage[/] for a small fix).",
            highlight=False,
        )


def register(app: typer.Typer) -> None:
    """Mount ``vibe init`` onto ``app`` (the shared wiring contract)."""
    app.command("init")(init_command)


__all__ = ["init_command", "register"]
