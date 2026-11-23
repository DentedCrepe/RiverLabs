//! Judging one of your decisions against the two stated references (docs/play.md):
//! the bots' preflop model, and pot odds against the bots' tracked ranges. The live
//! per-decision feedback and the end-of-session report both use this, so they always agree.

use crate::record::HeroDecision;
use poker_core::game::Action;
use serde::{Deserialize, Serialize};

/// A model frequency at or below this means "the model almost never does this".
pub const RARE: f64 = 0.10;
/// Equity margin beyond break-even before a call or fold is flagged: river / all-in…
pub const MARGIN_STRICT: f64 = 0.05;
/// …and flop / turn, where later betting matters and only large gaps are flagged.
pub const MARGIN_LOOSE: f64 = 0.15;

pub const MODEL_REFERENCE: &str = "GTOpen's preflop model for this table (an approximate model)";

/// One check a decision was eligible for.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub key: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    pub reference: &'static str,
    /// True when the decision is flagged by this check.
    pub hit: bool,
    pub text: String,
}

/// The model's view of a preflop decision, for the report's comparison line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelShares {
    pub you_voluntary: bool,
    pub you_raise: bool,
    pub model_voluntary: f64,
    pub model_raise: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Judgement {
    pub checks: Vec<Check>,
    pub model: Option<ModelShares>,
    /// Model frequencies for your hand at this spot ("Fold 20% · Raise 2.5 80%").
    pub model_mix: Option<String>,
}

/// Live feedback on one decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Feedback {
    /// "mistake", "ok" or "not_graded".
    pub verdict: String,
    pub headline: String,
    pub detail: String,
    pub reference: Option<String>,
}

fn is_aggressive(a: &Action) -> bool {
    matches!(a, Action::Bet { .. } | Action::Raise { .. })
}

/// `amount` formats chip amounts in the table's units.
pub fn judge(d: &HeroDecision, position: &str, amount: impl Fn(u64) -> String) -> Judgement {
    let mut j = Judgement::default();
    if let Some(m) = &d.model {
        let freq = |pred: &dyn Fn(&str) -> bool| -> f64 {
            m.actions.iter().zip(&m.frequencies).filter(|(a, _)| pred(&a.to_ascii_lowercase())).map(|(_, f)| f).sum()
        };
        let fold_f = freq(&|a| a == "fold");
        let check_f = freq(&|a| a == "check");
        let raise_f = freq(&|a| a.starts_with("raise") || a.starts_with("all-in") || a.starts_with("bet"));
        let you_vol = !matches!(d.action, Action::Fold | Action::Check);
        j.model = Some(ModelShares {
            you_voluntary: you_vol,
            you_raise: is_aggressive(&d.action),
            model_voluntary: 1.0 - fold_f - check_f,
            model_raise: raise_f,
        });
        j.model_mix = Some(
            m.actions
                .iter()
                .zip(&m.frequencies)
                .map(|(a, f)| format!("{a} {:.0}%", f * 100.0))
                .collect::<Vec<_>>()
                .join(" · "),
        );
        if let Some(t) = m.taken {
            let text = format!(
                "{position} {} with {}: you chose {} (model {:.0}%; model fold {:.0}%)",
                m.spot.replace('_', " "),
                m.class,
                m.actions[t],
                m.frequencies[t] * 100.0,
                fold_f * 100.0
            );
            if you_vol {
                j.checks.push(Check {
                    key: "preflop_too_loose",
                    title: "Played hands the model folds",
                    detail: "You put money in with hands the model folds at least 90% of the time in that spot.",
                    reference: MODEL_REFERENCE,
                    hit: fold_f >= 1.0 - RARE,
                    text: text.clone(),
                });
            }
            if d.action == Action::Fold {
                j.checks.push(Check {
                    key: "preflop_too_tight",
                    title: "Folded hands the model plays",
                    detail: "You folded hands the model plays at least 90% of the time in that spot.",
                    reference: MODEL_REFERENCE,
                    hit: fold_f <= RARE,
                    text: text.clone(),
                });
            }
            if d.action == Action::Call && raise_f >= 1.0 - RARE {
                j.checks.push(Check {
                    key: "preflop_passive",
                    title: "Called where the model raises",
                    detail: "You called with hands the model raises at least 90% of the time.",
                    reference: MODEL_REFERENCE,
                    hit: true,
                    text,
                });
            }
        }
    }
    // Preflop calls are judged against the model above; pot odds alone ignore position and
    // the play after the flop, so they only grade preflop calls that are all-in.
    let graded = d.street != "preflop" || d.all_in_call;
    if let (true, Some(req), Some(eq)) = (graded, d.required_equity, d.equity_vs_ranges) {
        let strict = d.all_in_call || d.street == "river";
        let margin = if strict { MARGIN_STRICT } else { MARGIN_LOOSE };
        let text = format!(
            "{} facing {} into {}: needed {:.0}% equity, had {:.0}% vs the range{}",
            d.street,
            amount(d.to_call),
            amount(d.pot),
            req * 100.0,
            eq * 100.0,
            if d.opponents > 1 { "s" } else { "" }
        );
        let reference = if strict {
            "Pot odds vs your equity against the bots' actual ranges (river or all-in: no later betting)"
        } else {
            "Pot odds vs your equity against the bots' actual ranges (flop/turn: ignores later betting, so only large gaps are flagged)"
        };
        match d.action {
            Action::Fold => j.checks.push(Check {
                key: if strict { "folded_with_odds" } else { "folded_with_odds_early" },
                title: if strict {
                    "Folded with enough equity (river/all-in)"
                } else {
                    "Folded with much more equity than needed (flop/turn)"
                },
                detail: "Your equity against the range you faced was above what the pot odds required.",
                reference,
                hit: eq >= req + margin,
                text,
            }),
            Action::Call => j.checks.push(Check {
                key: if strict { "called_without_odds" } else { "called_without_odds_early" },
                title: if strict {
                    "Called without enough equity (river/all-in)"
                } else {
                    "Called with much less equity than needed (flop/turn)"
                },
                detail: "Your equity against the range you faced was below what the pot odds required.",
                reference,
                hit: eq <= req - margin,
                text,
            }),
            _ => {}
        }
    }
    j
}

/// What to tell you straight after a decision.
///
/// Verdicts: "mistake" (flagged by a check), "rare" (the model almost never chooses this, though
/// it isn't one of the flagged patterns), "ok" (not flagged; the model chooses it often enough, or
/// the call or fold agrees with the pot odds), "not_graded" (no reference applies).
pub fn feedback(d: &HeroDecision, j: &Judgement) -> Feedback {
    let join = |parts: Vec<Option<String>>| parts.into_iter().flatten().collect::<Vec<_>>().join(" ");
    let mix = j.model_mix.as_ref().map(|m| format!("Model mix for your hand here: {m}."));
    if let Some(c) = j.checks.iter().find(|c| c.hit) {
        return Feedback {
            verdict: "mistake".into(),
            headline: c.title.into(),
            detail: join(vec![Some(format!("{}.", c.text)), mix]),
            reference: Some(c.reference.into()),
        };
    }
    let odds = j.checks.iter().find(|c| c.reference != MODEL_REFERENCE);
    if let Some(taken) = d.model.as_ref().and_then(|m| m.taken.map(|t| (m, t))) {
        let (m, t) = taken;
        let f = m.frequencies[t];
        let (verdict, headline) = if f >= 0.5 {
            ("ok", format!("The model's main choice ({:.0}%)", f * 100.0))
        } else if f > RARE {
            ("ok", format!("A choice the model also makes ({:.0}%)", f * 100.0))
        } else {
            ("rare", format!("The model rarely does this ({:.0}%)", f * 100.0))
        };
        return Feedback {
            verdict: verdict.into(),
            headline,
            detail: join(vec![mix, odds.map(|c| format!("{}.", c.text))]),
            reference: Some(MODEL_REFERENCE.into()),
        };
    }
    if let Some(c) = odds {
        return Feedback {
            verdict: "ok".into(),
            headline: "Consistent with the pot odds".into(),
            detail: format!("{}.", c.text),
            reference: Some(c.reference.into()),
        };
    }
    let why = if d.street == "preflop" && d.model.is_none() {
        "This spot is outside the bots' preflop model tree (e.g. after a limp or a 4-bet), so there is no model frequency to compare with."
    } else if is_aggressive(&d.action) {
        "Bets and raises after the flop aren't graded: that needs a solver for the exact spot."
    } else if d.action == Action::Check {
        "Checks aren't graded after the flop."
    } else {
        "Your equity against the bots' ranges couldn't be measured here."
    };
    Feedback {
        verdict: "not_graded".into(),
        headline: "Not graded".into(),
        detail: join(vec![Some(why.into()), mix]),
        reference: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::ModelCheck;

    fn decision(action: Action, folds: f64, street: &str) -> HeroDecision {
        HeroDecision {
            street: street.into(),
            action,
            to_call: 5_000,
            pot: 15_000,
            required_equity: Some(0.25),
            equity_vs_ranges: Some(0.40),
            opponents: 1,
            model: (street == "preflop").then(|| ModelCheck {
                spot: "vs_open".into(),
                class: "AKs".into(),
                actions: vec!["Fold".into(), "Call".into()],
                frequencies: vec![folds, 1.0 - folds],
                taken: Some(usize::from(action != Action::Fold)),
            }),
            all_in_call: false,
            feedback: None,
            spot: None,
        }
    }

    fn verdict(d: &HeroDecision) -> String {
        feedback(d, &judge(d, "BB", |x| x.to_string())).verdict
    }

    #[test]
    fn verdicts_follow_the_documented_thresholds() {
        // Folding a hand the model never folds: flagged.
        assert_eq!(verdict(&decision(Action::Fold, 0.0, "preflop")), "mistake");
        // Calling a hand the model calls 95% of the time: fine.
        assert_eq!(verdict(&decision(Action::Call, 0.05, "preflop")), "ok");
        // Calling a hand the model folds 95% of the time: flagged as too loose.
        assert_eq!(verdict(&decision(Action::Call, 0.95, "preflop")), "mistake");
        // Calling a hand the model folds 85%: not flagged (above the 10% "rare" line), but uncommon.
        assert_eq!(verdict(&decision(Action::Call, 0.85, "preflop")), "ok");
        // River: 40% equity against a 25% price. Folding gives up 15 points: flagged; calling is fine.
        assert_eq!(verdict(&decision(Action::Fold, 0.0, "river")), "mistake");
        assert_eq!(verdict(&decision(Action::Call, 0.0, "river")), "ok");
        // Flop: the same 15-point gap is exactly the looser margin, so a fold is flagged too.
        assert_eq!(verdict(&decision(Action::Fold, 0.0, "flop")), "mistake");
        // Bets after the flop are not graded.
        assert_eq!(verdict(&decision(Action::Bet { to: poker_core::Chips(5_000) }, 0.0, "turn")), "not_graded");
    }
}
