"""Unit tests for the active-rules digest (vibe-cli/10).

Cover the port's selection contract (cap at 5, pinned first, 📌 prefix, the
comment-block skip), the in-place block replacement, the symlink-aware
write-once behaviour, and the ``vibe rules`` command wiring via a LOCAL Typer
app (app.py is wired in unit 16, not here).
"""

from __future__ import annotations

import subprocess
import sys

import typer
from typer.testing import CliRunner

from vibe import markers, rules
from vibe.commands import rules_cmd

runner = CliRunner()


def _write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def _seed_target(root, body="placeholder"):
    """A file carrying an existing (stale) managed block to be replaced."""
    return (
        "# Guide\n\n"
        f"{markers.wrap(rules.MARKER_NAME, body)}\n\n"
        "User prose below the block.\n"
    )


def test_import_is_stdlib_only():
    code = (
        "import sys, vibe.rules;"
        "banned=[m for m in ('typer','rich','pydantic') if m in sys.modules];"
        "print(','.join(banned))"
    )
    proc = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    )
    assert proc.stdout.strip() == ""


def test_digest_capped_at_five_pinned_first(tmp_path, sample_lessons):
    _write(tmp_path / ".spec" / "lessons.md", sample_lessons)
    digest = rules.build_digest(tmp_path / ".spec" / "lessons.md")
    lines = digest.split("\n")

    # Six entries in the sample, capped to five.
    assert len(lines) == rules.CAP

    # Exactly the one pinned entry, and it sorts first with the 📌 prefix.
    pinned = [ln for ln in lines if ln.startswith("- \U0001f4cc ")]
    assert len(pinned) == 1
    assert lines[0].startswith("- \U0001f4cc ")
    assert "Preserve per-project runtime state across a re-copy" in lines[0]

    # Recency order among the unpinned entries: newest date first, oldest dropped.
    assert "Offer the superpower" in lines[1]
    assert "Scripts self-locate" in lines[2]
    assert "Uninstall must be surgical" in lines[3]
    assert "Single-source the per-turn orders" in lines[4]
    # The oldest (2026-06-06) falls off the cap.
    assert "Warn-first" not in digest


def test_digest_format_matches_origin_shape(tmp_path, sample_lessons):
    _write(tmp_path / ".spec" / "lessons.md", sample_lessons)
    digest = rules.build_digest(tmp_path / ".spec" / "lessons.md")
    # "- **title** — rule" (unpinned) / "- 📌 **title** — rule" (pinned).
    assert (
        "- \U0001f4cc **Preserve per-project runtime state across a re-copy** — "
        "Snapshot the cursor before a managed-file refresh and restore it after; "
        "seed only when absent."
    ) == digest.split("\n")[0]
    assert digest.split("\n")[1].startswith("- **Offer the superpower")


def test_comment_block_template_is_skipped(tmp_path):
    # A trailing HTML-comment template (as real lessons.md files carry) must not
    # contribute a phantom entry, even though it contains a "### " heading.
    lessons = (
        "# Lessons\n\n"
        "### Real entry\n"
        "**Rule:** Keep it real.\n"
        "**Date:** 2026-01-01\n\n"
        "<!-- Template — copy when adding an entry:\n"
        "### Fake heading inside a comment\n"
        "**Rule:** This must never appear in the digest.\n"
        "**Date:** 2099-12-31\n"
        "-->\n"
    )
    _write(tmp_path / ".spec" / "lessons.md", lessons)
    digest = rules.build_digest(tmp_path / ".spec" / "lessons.md")
    assert digest == "- **Real entry** — Keep it real."
    assert "Fake heading" not in digest


def test_empty_and_missing_degrade_gracefully(tmp_path):
    # Missing file.
    assert rules.build_digest(tmp_path / ".spec" / "lessons.md") == rules.EMPTY_DIGEST
    # Present but parses to nothing.
    _write(tmp_path / ".spec" / "lessons.md", "# Lessons\n\nNo entries here.\n")
    assert rules.build_digest(tmp_path / ".spec" / "lessons.md") == rules.EMPTY_DIGEST


def test_block_replaced_in_place(tmp_path, sample_lessons):
    _write(tmp_path / ".spec" / "lessons.md", sample_lessons)
    agents = tmp_path / "AGENTS.md"
    _write(agents, _seed_target(tmp_path, body="STALE CONTENT"))

    written = rules.write_rules(tmp_path, targets=[agents])
    assert written == [agents]

    text = agents.read_text(encoding="utf-8")
    # Exactly one managed region, and it no longer holds the stale body.
    assert text.count(markers.marker_lines(rules.MARKER_NAME)[0]) == 1
    assert text.count(markers.marker_lines(rules.MARKER_NAME)[1]) == 1
    assert "STALE CONTENT" not in text
    # User prose outside the markers survives byte-for-byte.
    assert text.startswith("# Guide\n")
    assert "User prose below the block." in text
    # The region equals build_block() exactly.
    region = markers.find_region(text, rules.MARKER_NAME)
    lines = text.split("\n")
    extracted = "\n".join(lines[region[0] : region[1] + 1])
    assert extracted == rules.build_block(tmp_path / ".spec" / "lessons.md")


def test_regenerate_is_idempotent(tmp_path, sample_lessons):
    _write(tmp_path / ".spec" / "lessons.md", sample_lessons)
    agents = tmp_path / "AGENTS.md"
    _write(agents, _seed_target(tmp_path))

    rules.write_rules(tmp_path, targets=[agents])
    once = agents.read_text(encoding="utf-8")
    rules.write_rules(tmp_path, targets=[agents])
    twice = agents.read_text(encoding="utf-8")
    assert once == twice


def test_symlinked_claude_written_once(tmp_path, sample_lessons):
    _write(tmp_path / ".spec" / "lessons.md", sample_lessons)
    agents = tmp_path / "AGENTS.md"
    claude = tmp_path / "CLAUDE.md"
    _write(agents, _seed_target(tmp_path))
    claude.symlink_to(agents.name)  # relative symlink, same dir

    # Default targets are [CLAUDE.md, AGENTS.md]; both resolve to AGENTS.md.
    written = rules.write_rules(tmp_path)
    assert written == [agents]

    # CLAUDE.md is still a symlink (not clobbered into a real file).
    assert claude.is_symlink()
    # AGENTS.md holds exactly one managed region (not doubled by the alias).
    text = agents.read_text(encoding="utf-8")
    assert text.count(markers.marker_lines(rules.MARKER_NAME)[0]) == 1


def test_both_real_targets_updated(tmp_path, sample_lessons):
    # The origin's primary path: two distinct real files, both refreshed.
    _write(tmp_path / ".spec" / "lessons.md", sample_lessons)
    agents = tmp_path / "AGENTS.md"
    claude = tmp_path / "CLAUDE.md"
    _write(agents, _seed_target(tmp_path, body="STALE-A"))
    _write(claude, _seed_target(tmp_path, body="STALE-C"))

    written = rules.write_rules(tmp_path)
    assert written == [claude, agents]

    expected = rules.build_block(tmp_path / ".spec" / "lessons.md")
    for target in (claude, agents):
        text = target.read_text(encoding="utf-8")
        region = markers.find_region(text, rules.MARKER_NAME)
        lines = text.split("\n")
        assert "\n".join(lines[region[0] : region[1] + 1]) == expected
        assert "STALE" not in text


def test_missing_target_is_skipped(tmp_path, sample_lessons):
    _write(tmp_path / ".spec" / "lessons.md", sample_lessons)
    # Neither CLAUDE.md nor AGENTS.md exists: nothing written, no crash.
    assert rules.write_rules(tmp_path) == []


def test_no_markers_appends_block(tmp_path, sample_lessons):
    _write(tmp_path / ".spec" / "lessons.md", sample_lessons)
    agents = tmp_path / "AGENTS.md"
    _write(agents, "# Guide\n\nSome prose, no markers.\n")

    rules.write_rules(tmp_path, targets=[agents])
    text = agents.read_text(encoding="utf-8")
    assert "Some prose, no markers." in text
    assert markers.has_region(text, rules.MARKER_NAME)
    region = markers.find_region(text, rules.MARKER_NAME)
    lines = text.split("\n")
    extracted = "\n".join(lines[region[0] : region[1] + 1])
    assert extracted == rules.build_block(tmp_path / ".spec" / "lessons.md")


def _local_app():
    app = typer.Typer()
    rules_cmd.register(app)
    return app


def test_command_updates_target(tmp_path, sample_lessons):
    _write(tmp_path / ".spec" / "lessons.md", sample_lessons)
    agents = tmp_path / "AGENTS.md"
    _write(agents, _seed_target(tmp_path, body="STALE"))

    result = runner.invoke(_local_app(), ["--root", str(tmp_path)])
    assert result.exit_code == 0
    assert "updated AGENTS.md" in result.output

    text = agents.read_text(encoding="utf-8")
    assert markers.has_region(text, rules.MARKER_NAME)
    assert "STALE" not in text


def test_command_warns_when_nothing_to_update(tmp_path, sample_lessons):
    _write(tmp_path / ".spec" / "lessons.md", sample_lessons)
    result = runner.invoke(_local_app(), ["--root", str(tmp_path)])
    assert result.exit_code == 0
    assert "no CLAUDE.md or AGENTS.md" in result.output


def test_command_warns_on_missing_lessons(tmp_path):
    agents = tmp_path / "AGENTS.md"
    _write(agents, _seed_target(tmp_path))
    result = runner.invoke(_local_app(), ["--root", str(tmp_path)])
    assert result.exit_code == 0
    assert "not found" in result.output
    # An empty digest is written when lessons.md is absent.
    text = agents.read_text(encoding="utf-8")
    assert rules.EMPTY_DIGEST in text
