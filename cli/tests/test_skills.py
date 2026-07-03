"""Tests for the bundled-skill copy engine (``vibe.provision.skills``)."""

from __future__ import annotations

from pathlib import Path

import vibe
from vibe.provision import skills

_ASSETS_SKILLS = Path(vibe.__file__).resolve().parent / "_assets" / "skills"


def _shipped_rel(name: str) -> set[Path]:
    src = _ASSETS_SKILLS / name
    return {p.relative_to(src) for p in src.rglob("*") if p.is_file()}


def test_import_pulls_no_heavy_deps() -> None:
    import subprocess
    import sys

    code = (
        "import sys, vibe.provision.skills; "
        "print(any(m in sys.modules for m in ('typer', 'rich', 'pydantic')))"
    )
    out = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    )
    assert out.stdout.strip() == "False", out.stdout + out.stderr


def test_install_copies_both_trees(target_project: Path) -> None:
    skills.install(target_project)
    dst_root = target_project / ".claude" / "skills"

    for name in ("spec", "vibe"):
        skill_dir = dst_root / name
        assert skill_dir.is_dir()
        for rel in _shipped_rel(name):
            copied = skill_dir / rel
            assert copied.is_file(), f"missing {name}/{rel}"
            assert copied.read_bytes() == (_ASSETS_SKILLS / name / rel).read_bytes()

    # Well-known anchors land where Claude Code discovers them.
    assert (dst_root / "spec" / "SKILL.md").is_file()
    assert (dst_root / "vibe" / "SKILL.md").is_file()
    assert (dst_root / "vibe" / "scripts" / "orders.sh").is_file()
    assert (dst_root / "spec" / "reference" / "templates" / "product.md").is_file()


def test_install_leaves_agents_untouched(target_project: Path) -> None:
    skills.install(target_project)
    assert not (target_project / ".agents").exists()


def test_install_returns_written_paths(target_project: Path) -> None:
    written = skills.install(target_project)
    expected = len(_shipped_rel("spec")) + len(_shipped_rel("vibe"))
    assert len(written) == expected
    assert all(p.is_file() for p in written)


def test_reinstall_is_idempotent(target_project: Path) -> None:
    first = sorted(str(p) for p in skills.install(target_project))
    dst = target_project / ".claude" / "skills" / "vibe" / "SKILL.md"
    before = dst.read_bytes()

    second = sorted(str(p) for p in skills.install(target_project))
    assert first == second
    assert dst.read_bytes() == before

    # No stray duplication or renaming: target file set matches the source set.
    for name in ("spec", "vibe"):
        skill_dir = target_project / ".claude" / "skills" / name
        on_disk = {p.relative_to(skill_dir) for p in skill_dir.rglob("*") if p.is_file()}
        assert on_disk == _shipped_rel(name)


def test_install_preserves_mode_bits(target_project: Path) -> None:
    skills.install(target_project)
    src = _ASSETS_SKILLS / "vibe" / "scripts" / "orders.sh"
    dst = target_project / ".claude" / "skills" / "vibe" / "scripts" / "orders.sh"
    assert (dst.stat().st_mode & 0o777) == (src.stat().st_mode & 0o777)


def test_dry_run_install_writes_nothing(target_project: Path) -> None:
    planned = skills.install(target_project, dry_run=True)
    assert planned  # a real plan is returned
    assert not (target_project / ".claude").exists()


def test_remove_is_surgical_and_prunes(target_project: Path) -> None:
    skills.install(target_project)
    dst_root = target_project / ".claude" / "skills"

    # Co-located user files INSIDE shipped subdirs — this is what discriminates a
    # surgical per-file remove from a naive `rm -rf skilldir`.
    user_in_scripts = dst_root / "vibe" / "scripts" / "USER_LOCAL.sh"
    user_in_scripts.write_text("#!/usr/bin/env bash\necho mine\n")
    user_at_root = dst_root / "spec" / "USER_NOTES.md"
    user_at_root.write_text("my notes\n")

    shipped_orders = dst_root / "vibe" / "scripts" / "orders.sh"
    shipped_spec_skill = dst_root / "spec" / "SKILL.md"
    assert shipped_orders.is_file() and shipped_spec_skill.is_file()

    removed = skills.remove(target_project)

    # Every shipped file is gone...
    for name in ("spec", "vibe"):
        for rel in _shipped_rel(name):
            assert not (dst_root / name / rel).exists(), f"shipped {name}/{rel} survived"
    assert set(str(p) for p in (shipped_orders, shipped_spec_skill)).issubset(
        str(p) for p in removed
    )

    # ...but the co-located user files (and their now non-empty dirs) survive.
    assert user_in_scripts.is_file()
    assert user_at_root.is_file()
    assert (dst_root / "vibe" / "scripts").is_dir()
    assert (dst_root / "spec").is_dir()

    # A directory that became truly empty is pruned.
    assert not (dst_root / "vibe" / "reference").exists()


def test_remove_prunes_all_when_no_user_files(target_project: Path) -> None:
    skills.install(target_project)
    skills.remove(target_project)
    # Nothing co-located: the whole skills subtree (and .claude) is pruned away.
    assert not (target_project / ".claude").exists()


def test_remove_on_fresh_target_is_noop(target_project: Path) -> None:
    removed = skills.remove(target_project)
    assert removed == []
    assert not (target_project / ".claude").exists()


def test_dry_run_remove_writes_nothing(target_project: Path) -> None:
    skills.install(target_project)
    dst_root = target_project / ".claude" / "skills"
    before = {p for p in dst_root.rglob("*")}

    planned = skills.remove(target_project, dry_run=True)
    assert planned  # would-remove list is populated
    after = {p for p in dst_root.rglob("*")}
    assert before == after  # nothing actually deleted
