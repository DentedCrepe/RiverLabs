"""The engine adapter: error translation and Rust <-> Pydantic schema agreement."""

import json

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

from riverlab_api.errors import EngineValidationError
from riverlab_api.schemas.config import GameConfigDoc
from riverlab_api.services import engine


def test_engine_errors_become_validation_errors() -> None:
    with pytest.raises(EngineValidationError, match="duplicate card"):
        engine.evaluate_hand("AsAsKdQh2c")
    with pytest.raises(EngineValidationError):
        engine.analyze_range("AAs")


def test_default_game_config_matches_pydantic_schema_exactly() -> None:
    """Guards against drift between the Rust serde types and the Pydantic transport models."""
    default = engine.default_game_config()
    doc = GameConfigDoc.model_validate(default)
    assert doc.model_dump() == default
    assert engine.validate_game_config(doc.model_dump()) == default


def test_unknown_fields_are_rejected_by_the_engine() -> None:
    cfg = engine.default_game_config()
    cfg["rake"]["surprise"] = 1
    with pytest.raises(EngineValidationError, match="unknown field"):
        engine.validate_game_config(cfg)


def test_positions_and_presets() -> None:
    assert [p["code"] for p in engine.positions(6)] == ["UTG", "HJ", "CO", "BTN", "SB", "BB"]
    presets = engine.stack_presets()
    assert [p["big_blinds"] for p in presets["presets"]] == [20, 40, 50, 75, 100, 150, 200]
    assert presets["default"] == "100bb"


def test_rake_defaults() -> None:
    cfg = engine.default_game_config()
    assert engine.compute_rake(cfg, 20_000, True) == {"rake": 1000, "capped": False, "reason": "percentage"}
    assert engine.compute_rake(cfg, 20_000, False)["reason"] == "no_flop_no_drop"


rake_amount = st.one_of(
    st.builds(lambda m: {"unit": "big_blinds", "mbb": m}, st.integers(0, 20_000)),
    st.builds(lambda m: {"unit": "currency", "minor_units": m}, st.integers(0, 2_000)),
)


@st.composite
def game_configs(draw: st.DrawFn) -> dict[str, object]:
    return {
        "schema_version": 1,
        "players": draw(st.integers(1, 8)),
        "small_blind": draw(st.integers(0, 1_200)),
        "ante": draw(st.integers(0, 1_200)),
        "stack": {"effective_stack": draw(st.integers(0, 1_200_000))},
        "rake": {
            "percentage_bps": draw(st.integers(0, 12_000)),
            "cap": draw(st.none() | rake_amount),
            "min_pot": draw(st.none() | rake_amount),
            "rounding": {
                "increment": draw(rake_amount),
                "mode": draw(st.sampled_from(["down", "nearest", "up"])),
            },
            "no_flop_no_drop": draw(st.booleans()),
            "allocation": draw(st.sampled_from(["proportional", "main_pot_first"])),
        },
        "stake": draw(
            st.none()
            | st.builds(
                lambda c, b: {"currency": c, "big_blind_minor_units": b},
                st.sampled_from(["USD", "EUR", "usd"]),
                st.integers(0, 500),
            )
        ),
    }


@settings(max_examples=300, deadline=None)
@given(game_configs(), st.integers(0, 2_000_000), st.booleans())
def test_config_validation_is_idempotent_and_rake_is_bounded(
    cfg: dict[str, object], pot: int, saw_flop: bool
) -> None:
    """Any config is either rejected with a message or accepted in a canonical form that
    re-validates to itself; accepted configs never produce rake above the pot or cap."""
    try:
        canonical = engine.validate_game_config(cfg)
    except EngineValidationError as exc:
        message = exc.message
    else:
        message = None
    if message is not None:
        assert message, "rejections must explain themselves"
        return
    assert engine.validate_game_config(canonical) == canonical
    GameConfigDoc.model_validate(canonical)
    result = engine.compute_rake(canonical, pot, saw_flop)
    assert 0 <= result["rake"] <= pot


@settings(max_examples=60, deadline=None)
@given(st.lists(st.sampled_from(["AA", "KK", "AKs", "AKo", "T9s", "72o", "QJ", "55-22", "ATs+"]), min_size=1))
def test_range_notation_round_trip(tokens: list[str]) -> None:
    text = ",".join(tokens)
    a = engine.analyze_range(text)
    b = engine.analyze_range(a["notation"])
    assert a["combos"] == b["combos"]
    assert a["notation"] == b["notation"]
    assert sum(c["weight_sum"] for c in a["matrix"]) == pytest.approx(a["combos"])


def test_equity_results_are_well_formed() -> None:
    r = engine.equity(["AsAh", "KsKh"], "2c7d9h", "", "exact", 1, 0, 10_000_000)
    assert r["method"] == "exact"
    assert sum(r["equity"]) == pytest.approx(1.0)
    assert r["samples"] == 990  # C(45, 2) turn+river runouts
    json.dumps(r)
