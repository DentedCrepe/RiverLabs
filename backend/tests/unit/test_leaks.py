"""How solver-review decisions are described as leaks."""

from riverlab_api.services.leaks import category, kind_of


def test_mistakes_are_named_by_what_you_did_and_what_was_better() -> None:
    assert kind_of("raise to 20bb") == "raise"
    assert kind_of("bet 6.6bb") == "bet"
    assert category("call 5bb", "fold") == "Called when folding was better"
    assert category("check", "bet 3.3bb") == "Checked when betting was better"
    assert category("fold", "raise to 12bb") == "Folded when raising was better"
    assert category("bet 3.3bb", "bet 7.5bb") == "Bet the wrong size"
    assert category("raise to 9bb", "raise to 20bb") == "Raised the wrong size"
