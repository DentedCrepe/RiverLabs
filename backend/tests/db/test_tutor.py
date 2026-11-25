"""The AI tutor end to end through the API, with a scripted stand-in for Claude: tools run against
the real engine and database, the number check and the reference check apply, and nothing is
sent anywhere without consent. No network or API key is needed."""

import json
from collections.abc import Iterator
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest
from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from riverlab_api.services import academy, tutor

pytestmark = pytest.mark.db
CONTENT = Path(__file__).resolve().parents[3] / "content" / "academy"


def _text(answer: str, sections: list[str] | None = None, readings: list[str] | None = None) -> Any:
    body = {
        "answer": answer,
        "sections": sections or [],
        "readings": readings or [],
        "suggested_lessons": ["equity", "not-a-lesson"],
    }
    return SimpleNamespace(type="text", text=json.dumps(body))


def _tool(name: str, args: dict[str, Any], i: int) -> Any:
    return SimpleNamespace(type="tool_use", id=f"t{i}", name=name, input=args)


def _reply(stop: str, *blocks: Any) -> Any:
    usage = SimpleNamespace(input_tokens=100, output_tokens=50, cache_read_input_tokens=0)
    return SimpleNamespace(stop_reason=stop, content=list(blocks), model="claude-opus-5-5", usage=usage)


class FakeClaude:
    """Returns scripted responses in order and records every request."""

    def __init__(self, script: list[Any]) -> None:
        self.script = list(script)
        self.requests: list[dict[str, Any]] = []
        self.beta = SimpleNamespace(messages=SimpleNamespace(create=self.create))

    def create(self, **kwargs: Any) -> Any:
        self.requests.append(json.loads(json.dumps(kwargs, default=lambda o: getattr(o, "__dict__", str(o)))))
        return self.script.pop(0)


@pytest.fixture
def synced(db: Session) -> Session:
    assert academy.sync(db, CONTENT).errors == []
    return db


@pytest.fixture
def fake(monkeypatch: pytest.MonkeyPatch) -> list[FakeClaude]:
    """Each tutor request gets a FakeClaude running the next script in `fake_scripts()`."""
    made: list[FakeClaude] = []

    def factory() -> FakeClaude:
        c = FakeClaude(_SCRIPTS.pop(0))
        made.append(c)
        return c

    monkeypatch.setattr(tutor, "client_factory", factory)
    return made


def ask(client: TestClient, question: str, consent: bool = True) -> Any:
    return client.post(
        "/api/v1/academy/lessons/pot-odds/tutor",
        json={"level": "intermediate", "question": question, "history": [], "consent": consent},
    )


def test_numbers_come_from_tools_and_untraced_ones_are_fixed(
    synced: Session, db_client: TestClient, fake: list[FakeClaude]
) -> None:
    fake_scripts().append(
        [
            # Asks the engine for the price of a half-pot bet, then answers with its numbers.
            _reply(
                "tool_use",
                _tool("theory", {"kind": "pot_odds", "params": {"pot_mbb": 15000, "to_call_mbb": 5000}}, 1),
            ),
            # 37% is not traceable: the check asks for a correction...
            _reply(
                "end_turn", _text("You need 25% equity; most players call 37% of the time.", ["break-even"])
            ),
            # ...and the corrected answer uses only the engine's figure.
            _reply(
                "end_turn",
                _text(
                    "You need 25% equity (RiverLab's engine, pot odds model).",
                    ["break-even", "made-up-section"],
                    ["The Mathematics of Poker", "An Invented Book"],
                ),
            ),
        ]
    )
    r = ask(db_client, "What equity do I need to call a half-pot bet?")
    assert r.status_code == 200, r.json()
    out = r.json()
    assert out["answer"].startswith("You need 25% equity")
    assert out["unverified_numbers"] == []
    assert out["corrections"] == 1
    assert out["tool_calls"][0]["tool"] == "theory"
    assert out["tool_calls"][0]["ok"]
    assert out["sections"] == ["break-even"], "unknown section ids are dropped"
    assert out["readings"] == ["The Mathematics of Poker"]
    assert out["dropped_readings"] == ["An Invented Book"], "references outside the reading lists are dropped"
    assert out["suggested_lessons"] == ["equity"]
    assert out["label"] == "AI tutor (may be wrong)"
    reqs = fake[0].requests
    assert len(reqs) == 3
    # The engine's result went back to the model, and the correction named the untraced number.
    tool_result = reqs[1]["messages"][-1]["content"][0]
    assert tool_result["type"] == "tool_result"
    assert "0.25" in tool_result["content"]
    assert "37" in reqs[2]["messages"][-1]["content"]
    # Grounding: the lesson at the learner's level, with stable instructions cached first.
    first = reqs[0]
    assert first["model"] == "claude-opus-5-5"
    assert first["system"][0]["cache_control"] == {"type": "ephemeral"}
    assert "Explanation level: intermediate" in first["messages"][0]["content"][0]["text"]
    assert "{#break-even}" in first["messages"][0]["content"][0]["text"]
    assert first["output_config"]["format"]["type"] == "json_schema"
    assert {t["name"] for t in first["tools"]} >= {"theory", "equity", "solver_result", "arithmetic"}


def test_numbers_still_untraced_after_a_correction_are_flagged(
    synced: Session, db_client: TestClient, fake: list[FakeClaude]
) -> None:
    fake_scripts().append(
        [
            _reply("end_turn", _text("The answer is 61.3%.")),
            _reply("end_turn", _text("The answer is 61.3%.")),
        ]
    )
    out = ask(db_client, "Tell me a number").json()
    assert out["unverified_numbers"] == ["61.3"]
    assert "could not trace these figures" in out["answer"]


def test_tool_errors_go_back_to_the_model(
    synced: Session, db_client: TestClient, fake: list[FakeClaude]
) -> None:
    fake_scripts().append(
        [
            _reply(
                "tool_use",
                _tool("equity", {"ranges": ["AsKd", "QhQc"], "board": ""}, 1),
                _tool("solver_result", {"job_id": "00000000-0000-0000-0000-000000000000"}, 2),
                _tool("arithmetic", {"expression": "__import__('os')"}, 3),
            ),
            _reply("end_turn", _text("AK has 42.84% against QQ (RiverLab's engine, exact).")),
        ]
    )
    out = ask(db_client, "AK against QQ?").json()
    calls = {c["tool"]: c for c in out["tool_calls"]}
    assert calls["equity"]["ok"]
    assert not calls["solver_result"]["ok"]
    assert "no stored result" in calls["solver_result"]["error"]
    assert not calls["arithmetic"]["ok"]
    assert out["unverified_numbers"] == [], "42.84 came from the exact equity tool"
    results = fake[0].requests[1]["messages"][-1]["content"]
    assert [r.get("is_error", False) for r in results] == [False, True, True]


def test_nothing_is_sent_without_consent_or_a_key(
    synced: Session, db_client: TestClient, fake: list[FakeClaude], monkeypatch: pytest.MonkeyPatch
) -> None:
    r = ask(db_client, "Hello?", consent=False)
    assert r.status_code == 403
    assert r.json()["error"]["code"] == "tutor_consent_required"
    assert fake == [], "no client was even created"
    monkeypatch.delenv("ANTHROPIC_API_KEY", raising=False)
    monkeypatch.delenv("ANTHROPIC_AUTH_TOKEN", raising=False)
    from riverlab_api.config import get_settings

    if get_settings().anthropic_api_key is None:
        s = db_client.get("/api/v1/academy/tutor").json()
        assert s["available"] is False
        assert "ANTHROPIC_API_KEY" in s["reason"]


_SCRIPTS: list[list[Any]] = []


def fake_scripts() -> list[list[Any]]:
    return _SCRIPTS


@pytest.fixture(autouse=True)
def _reset_scripts(monkeypatch: pytest.MonkeyPatch) -> Iterator[None]:
    _SCRIPTS.clear()
    yield
    _SCRIPTS.clear()
