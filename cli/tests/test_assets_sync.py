"""Drift guard: every vendored ``_assets`` file must match its repo source (vibe-cli/16).

The CLI provisions from ``src/vibe/_assets/`` (so ``vibe init`` needs no repo clone). Those
files are byte-copies of ``flow/`` and ``spec/``; this test fails the moment a source drifts
from its bundled copy, so the retirement of the bash sources cannot silently desync the CLI.
Both ``state-machine.json`` copies (the flat one for ``machine.py`` and the one inside the
bundled ``vibe`` skill tree) are checked against the single ``flow/state-machine.json`` source.
"""

from __future__ import annotations

from pathlib import Path

import vibe

REPO = Path(__file__).resolve().parents[2]
ASSETS = Path(vibe.__file__).resolve().parent / "_assets"

_FLAT = {
    "state-machine.json": REPO / "flow" / "state-machine.json",
    "state.example.json": REPO / "flow" / "state.example.json",
    "deps.json": REPO / "flow" / "reference" / "deps.json",
    "adapters.json": REPO / "flow" / "reference" / "adapters.json",
}


def _source_for(rel: Path) -> Path | None:
    parts = rel.parts
    if parts[0] == "skills":
        base = REPO / ("flow" if parts[1] == "vibe" else "spec")
        return base.joinpath(*parts[2:])
    if parts[0] == "templates":
        return REPO / "flow" / "reference" / "templates" / Path(*parts[1:])
    return _FLAT.get(rel.as_posix())


def test_every_bundled_asset_matches_its_source():
    assert ASSETS.is_dir(), f"missing bundled assets at {ASSETS}"
    mismatches: list[str] = []
    checked = 0
    for path in sorted(ASSETS.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(ASSETS)
        source = _source_for(rel)
        if source is None:
            mismatches.append(f"{rel}: no known source mapping")
            continue
        if not source.is_file():
            mismatches.append(f"{rel}: source missing at {source}")
            continue
        if path.read_bytes() != source.read_bytes():
            mismatches.append(f"{rel}: differs from {source}")
            continue
        checked += 1
    assert not mismatches, "asset drift:\n" + "\n".join(mismatches)
    assert checked > 0


def test_both_state_machine_copies_match_source():
    source = (REPO / "flow" / "state-machine.json").read_bytes()
    flat = (ASSETS / "state-machine.json").read_bytes()
    bundled = (ASSETS / "skills" / "vibe" / "state-machine.json").read_bytes()
    assert flat == source
    assert bundled == source
