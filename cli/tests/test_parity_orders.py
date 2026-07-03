"""Byte-for-byte parity: ``orders.resolve`` vs ``flow/scripts/orders.sh``.

The merge gate for unit 5 — the bundled Python resolver must reproduce the bash
origin's stdout exactly for every state, with and without a ``<feature>``. The
bash script always interpolates the feature from the live cursor
(``flow/state.json``) regardless of the state arg, so this test mirror-reads that
same file via :mod:`vibe.cursor` and passes the identical feature to
``resolve`` — holding the feature source equal on both sides so the comparison
isolates the extraction/tier logic. Skips when ``bash``/``jq`` are unavailable
(handled by the ``bash_ref`` fixture).
"""

from __future__ import annotations

from pathlib import Path

import pytest

from vibe import cursor, orders

_REPO_ROOT = Path(__file__).resolve().parents[2]
_FLOW_STATE = _REPO_ROOT / "flow" / "state.json"

# States exercised in addition to every machine state: unknown keys that fall
# through to the generic one-liner (locks the hand-typed GENERIC_FALLBACK bytes).
_FALLBACK_STATES = ["bogus.state", "totally-unknown"]


def _cursor_feature() -> str | None:
    """The feature ``orders.sh`` would interpolate — read exactly as it does."""
    if not _FLOW_STATE.exists():
        return None
    try:
        cur = cursor.read(_FLOW_STATE)
    except Exception:
        return None
    return cur.feature if cur is not None else None


def _state_keys(machine_data) -> list[str]:
    return list(machine_data["states"].keys()) + _FALLBACK_STATES


def test_parity_every_state(bash_ref, machine_data):
    feature = _cursor_feature()
    for state in _state_keys(machine_data):
        result = bash_ref("orders.sh", state)
        expected = orders.resolve(state, feature) + "\n"
        assert result.stdout == expected, f"parity drift for state {state!r}"
        assert result.returncode == 0


def test_parity_current_cursor_no_arg(bash_ref):
    feature = _cursor_feature()
    state = cursor.current_state(_FLOW_STATE)
    result = bash_ref("orders.sh")
    assert result.stdout == orders.resolve(state, feature) + "\n"
    assert result.returncode == 0


def test_parity_generic_fallback_bytes(bash_ref):
    result = bash_ref("orders.sh", "no-such-state.here")
    assert result.stdout == orders.GENERIC_FALLBACK + "\n"


@pytest.mark.parametrize("state", ["feature.impl", "feature.plan", "feature.compound"])
def test_parity_feature_interpolated_states(bash_ref, state):
    feature = _cursor_feature()
    result = bash_ref("orders.sh", state)
    assert result.stdout == orders.resolve(state, feature) + "\n"
