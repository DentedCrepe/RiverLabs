//! Bot decisions. Two sources, both documented in `docs/play.md`:
//!
//! - **Preflop:** GTOpen's preflop model (an approximate model). The bot looks up its hand class
//!   at the model node matching the hand so far and samples an action from the model's
//!   frequencies. Lower difficulties add named leaks (see [`preflop_frequencies`]).
//! - **Postflop, and preflop once a hand leaves the model tree:** transparent rules. A bot ranks
//!   every combo still in its own range by strength against a random hand on this board, then
//!   acts on its percentile: bet the top `value` share, bluff with the bottom share given by the
//!   bet-size ratio, defend the minimum-defence share against a bet, raise the top few percent.
//!
//! Every decision also returns the probability of the chosen action for **every combo**, so the
//! session can track each bot's exact range (used to grade your calls and folds).

use crate::config::Difficulty;
use crate::strength::{percentiles, preflop_class_strength, range_strengths, top_share_strength};
use poker_core::game::{Action, LegalActions};
use poker_core::{Card, Chips, HoleCards};
use rand::RngExt;
use rand_chacha::ChaCha8Rng;
use range_engine::{COMBO_COUNT, Combo, HandClass};
use solver_gtopen::preflop::PreflopNode;

/// What the bot did, and with what probability each combo would have done the same.
#[derive(Debug, Clone)]
pub struct BotDecision {
    pub action: Action,
    /// Per combo index: probability of taking this (abstract) action. Multiplies the bot's range.
    pub combo_probability: Vec<f64>,
    /// Index of the model action taken, when the decision came from the preflop model.
    pub model_action: Option<usize>,
    pub source: DecisionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionSource {
    PreflopModel,
    PreflopRules,
    PostflopRules,
}

/// Everything a bot sees when it acts.
#[derive(Debug)]
pub struct Situation<'a> {
    pub hole: HoleCards,
    pub board: &'a [Card],
    pub legal: LegalActions,
    pub difficulty: Difficulty,
    /// Pot including all bets so far.
    pub pot: Chips,
    /// The bot's chips behind plus what it has in on this street.
    pub stack_to: Chips,
    pub current_bet: Chips,
    pub big_blind: Chips,
    /// Players still in the hand, the bot included.
    pub live_players: usize,
    /// The bot's own range so far (weights per combo index).
    pub range: &'a [f64],
    pub seed: u64,
}

fn class_of(c: Combo) -> HandClass {
    HandClass::of(c)
}

fn hole_combo(h: HoleCards) -> Combo {
    let [a, b] = h.cards();
    Combo::new(a, b).expect("distinct hole cards")
}

/// Model frequencies for `class` at `node`, adjusted for difficulty. The leaks:
/// - **Medium:** facing a raise, a quarter of the model's folds become calls.
/// - **Easy:** facing a raise, half of the folds become calls with any hand in the top 60% of
///   hands by strength vs a random hand; re-raises happen only with the top 5% of hands.
pub fn preflop_frequencies(node: &PreflopNode, class: HandClass, difficulty: Difficulty) -> Option<Vec<f64>> {
    let label = class.to_string();
    let cs = node.by_class.iter().find(|c| c.label == label)?;
    let mut f = cs.frequencies.clone();
    let fold = node.actions.iter().position(|a| a.kind == "fold");
    let call = node.actions.iter().position(|a| a.kind == "call");
    if let (Some(fi), Some(ci)) = (fold, call) {
        let strength = preflop_class_strength(class);
        let (moved, raises_ok) = match difficulty {
            Difficulty::Hard => (0.0, true),
            Difficulty::Medium => (0.25 * f[fi], true),
            Difficulty::Easy => (
                if strength >= top_share_strength(0.6) { 0.5 * f[fi] } else { 0.0 },
                strength >= top_share_strength(0.05),
            ),
        };
        f[fi] -= moved;
        f[ci] += moved;
        if !raises_ok {
            for (i, a) in node.actions.iter().enumerate() {
                if a.kind != "fold" && a.kind != "call" && a.kind != "check" {
                    f[ci] += f[i];
                    f[i] = 0.0;
                }
            }
        }
    }
    let total: f64 = f.iter().sum();
    if total <= 0.0 {
        return None;
    }
    Some(f.into_iter().map(|x| x / total).collect())
}

fn sample(probs: &[f64], rng: &mut ChaCha8Rng) -> usize {
    let r: f64 = rng.random();
    let mut acc = 0.0;
    for (i, p) in probs.iter().enumerate() {
        acc += p;
        if r < acc {
            return i;
        }
    }
    probs.len() - 1
}

fn to_raise(legal: &LegalActions, to: Chips, stack_to: Chips) -> Action {
    let (bounds, is_bet) = match (legal.bet, legal.raise) {
        (Some(b), _) => (b, true),
        (None, Some(r)) => (r, false),
        (None, None) => return if legal.call.is_some() { Action::Call } else { Action::Check },
    };
    let (min, max) = bounds;
    // Within 15% of the stack: just go all-in.
    let to = if to.mbb() * 100 >= stack_to.mbb() * 85 { max } else { to.max(min).min(max) };
    if is_bet { Action::Bet { to } } else { Action::Raise { to } }
}

fn passive(legal: &LegalActions) -> Action {
    if legal.check { Action::Check } else { Action::Call }
}

fn fold_or_check(legal: &LegalActions) -> Action {
    if legal.check { Action::Check } else { Action::Fold }
}

/// Maps the model's action `a` at `node` to a real action. `unit_per_bb` converts model bb.
fn model_to_real(node: &PreflopNode, a: usize, s: &Situation) -> Action {
    let act = &node.actions[a];
    match act.kind.as_str() {
        "fold" => fold_or_check(&s.legal),
        "check" | "call" => passive(&s.legal),
        "jam" => to_raise(&s.legal, s.stack_to, s.stack_to),
        _ => {
            let to = Chips((act.to_mbb / 1000.0 * s.big_blind.mbb() as f64).round() as u64);
            to_raise(&s.legal, to, s.stack_to)
        }
    }
}

/// Preflop decision from the model. `None` when the bot's class is not in the node's strategy.
pub fn preflop_model(node: &PreflopNode, s: &Situation, rng: &mut ChaCha8Rng) -> Option<BotDecision> {
    let mine = preflop_frequencies(node, class_of(hole_combo(s.hole)), s.difficulty)?;
    let a = sample(&mine, rng);
    let mut probs = vec![0.0; COMBO_COUNT];
    for (i, p) in probs.iter_mut().enumerate() {
        let c = Combo::from_index(i).expect("combo");
        if s.range[i] > 0.0 {
            *p = preflop_frequencies(node, class_of(c), s.difficulty).map_or(0.0, |f| f[a]);
        }
    }
    Some(BotDecision {
        action: model_to_real(node, a, s),
        combo_probability: probs,
        model_action: Some(a),
        source: DecisionSource::PreflopModel,
    })
}

/// Difficulty parameters of the rules (documented in `docs/play.md`).
#[derive(Debug, Clone, Copy)]
pub struct RuleParams {
    /// Share of the range bet for value when not facing a bet (heads-up).
    pub value: f64,
    /// Multiplier on the bluffs implied by the bet size (1 = balanced ratio, 0 = never bluff).
    pub bluff: f64,
    /// Bet size as a fraction of the pot.
    pub bet_size: f64,
    /// Multiplier on the minimum-defence share when facing a bet (>1 calls too much).
    pub defend: f64,
    /// Share of the range that raises for value when facing a bet.
    pub raise: f64,
}

pub fn params(d: Difficulty) -> RuleParams {
    match d {
        Difficulty::Hard => RuleParams { value: 0.33, bluff: 1.0, bet_size: 0.66, defend: 1.0, raise: 0.08 },
        Difficulty::Medium => RuleParams { value: 0.28, bluff: 0.5, bet_size: 0.5, defend: 0.9, raise: 0.06 },
        Difficulty::Easy => RuleParams { value: 0.2, bluff: 0.0, bet_size: 0.4, defend: 1.3, raise: 0.02 },
    }
}

/// Abstract choices of the rules: 0 = fold (or check when free), 1 = check/call, 2 = bet/raise.
fn rule_choice(p: f64, facing: bool, value: f64, bluff: f64, defend: f64, raise: f64) -> usize {
    if facing {
        if p >= 1.0 - raise {
            2
        } else if p >= 1.0 - defend {
            1
        } else {
            0
        }
    } else if p >= 1.0 - value || p < bluff {
        2
    } else {
        1
    }
}

/// Rule-based decision (postflop, or preflop off the model tree).
pub fn rules(s: &Situation, rng_seed: u64) -> BotDecision {
    let preflop = s.board.is_empty();
    let par = params(s.difficulty);
    let facing = s.legal.call.is_some();
    let multiway = s.live_players > 2;

    // Strength of every combo in the range.
    let strength: Vec<f64> = if preflop {
        (0..COMBO_COUNT)
            .map(|i| {
                if s.range[i] > 0.0 {
                    preflop_class_strength(class_of(Combo::from_index(i).expect("combo")))
                } else {
                    0.0
                }
            })
            .collect()
    } else {
        range_strengths(s.range, s.board, 150, rng_seed)
    };
    // Combos that use a board card are impossible.
    let mut weights = s.range.to_vec();
    for (i, w) in weights.iter_mut().enumerate() {
        let (a, b) = Combo::from_index(i).expect("combo").cards();
        if s.board.contains(&a) || s.board.contains(&b) {
            *w = 0.0;
        }
    }
    let pct = percentiles(&weights, &strength);

    let pot = s.pot.mbb() as f64;
    let to_call = s.legal.call.map_or(0.0, |c| c.mbb() as f64);
    // Minimum defence share against a bet of `to_call` into the pot before it.
    let mdf = if facing { ((pot - to_call) / pot.max(1.0)).clamp(0.0, 1.0) } else { 0.0 };
    let mut defend = (mdf * par.defend).clamp(0.05, 1.0);
    let mut value = par.value;
    if multiway {
        defend *= 0.8;
        value *= 0.6;
    }
    if preflop {
        // Off the model tree preflop: tighter defaults (no pot-odds defence of wide ranges).
        defend = defend.min(match s.difficulty {
            Difficulty::Hard => 0.2,
            Difficulty::Medium => 0.3,
            Difficulty::Easy => 0.45,
        });
    }
    // Bluffs balance the value bets at the bet size's ratio B/(P+2B) (river indifference).
    let ratio = par.bet_size / (1.0 + 2.0 * par.bet_size);
    let bluff = if preflop { 0.0 } else { value * ratio / (1.0 - ratio) * par.bluff };

    let probs_of = |choice: usize| -> Vec<f64> {
        (0..COMBO_COUNT)
            .map(|i| {
                if weights[i] <= 0.0 {
                    0.0
                } else {
                    f64::from(u8::from(rule_choice(pct[i], facing, value, bluff, defend, par.raise) == choice))
                }
            })
            .collect()
    };
    let me = hole_combo(s.hole).index();
    let choice = rule_choice(pct[me], facing, value, bluff, defend, par.raise);
    let action = match choice {
        0 => fold_or_check(&s.legal),
        1 => passive(&s.legal),
        _ => {
            let to = if facing { Chips(s.current_bet.mbb() * 3) } else { Chips((pot * par.bet_size).round() as u64) };
            to_raise(&s.legal, to, s.stack_to)
        }
    };
    BotDecision {
        action,
        combo_probability: probs_of(choice),
        model_action: None,
        source: if preflop { DecisionSource::PreflopRules } else { DecisionSource::PostflopRules },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn legal(call: Option<u64>, bet: Option<(u64, u64)>, raise: Option<(u64, u64)>) -> LegalActions {
        LegalActions {
            seat: 0,
            fold: call.is_some(),
            check: call.is_none(),
            call: call.map(Chips),
            bet: bet.map(|(a, b)| (Chips(a), Chips(b))),
            raise: raise.map(|(a, b)| (Chips(a), Chips(b))),
        }
    }

    fn hole(s: &str) -> HoleCards {
        let c = Card::parse_many(s).unwrap();
        HoleCards::new(c[0], c[1]).unwrap()
    }

    #[test]
    fn rules_bet_strong_hands_and_defend_by_pot_odds() {
        let board = Card::parse_many("Ks7d2c").unwrap();
        let range = vec![1.0; COMBO_COUNT];
        let mk = |h: &str, l: LegalActions, d| Situation {
            hole: hole(h),
            board: &board,
            legal: l,
            difficulty: d,
            pot: Chips(10_000),
            stack_to: Chips(90_000),
            current_bet: Chips(l.call.map_or(0, |c| c.mbb())),
            big_blind: Chips(1000),
            live_players: 2,
            range: &range,
            seed: 7,
        };
        let open = legal(None, Some((1000, 90_000)), None);
        let set = rules(&mk("7h7s", open, Difficulty::Hard), 1);
        assert!(matches!(set.action, Action::Bet { to } if to == Chips(6_600)), "{:?}", set.action);
        // Easy never bluffs: with air it checks.
        assert_eq!(rules(&mk("4h3h", open, Difficulty::Easy), 1).action, Action::Check);
        // Facing a pot-sized bet (pot 10bb includes the 5bb bet): defend half the range.
        let facing = legal(Some(5_000), None, Some((10_000, 90_000)));
        let d = rules(&mk("7h7s", facing, Difficulty::Hard), 1);
        assert!(matches!(d.action, Action::Raise { .. }), "{:?}", d.action);
        assert_eq!(rules(&mk("4h3h", facing, Difficulty::Hard), 1).action, Action::Fold);
        // The range probabilities describe the same policy: about half the range continues.
        let call = rules(&mk("Qh8h", facing, Difficulty::Hard), 1);
        let continuing: f64 = (0..COMBO_COUNT)
            .filter(|&i| {
                let (a, b) = Combo::from_index(i).unwrap().cards();
                !board.contains(&a) && !board.contains(&b)
            })
            .map(|i| call.combo_probability[i])
            .sum();
        assert!(continuing > 0.0);
    }

    #[test]
    fn preflop_leaks_shift_folds_to_calls() {
        let node: PreflopNode = serde_json::from_value(serde_json::json!({
            "kind": "action", "actor": "BB", "positions": ["SB", "BB"], "pot_mbb": 3000.0,
            "invested_mbb": [2000.0, 1000.0], "live": [true, true],
            "actions": [
                {"label": "Fold", "kind": "fold", "to_mbb": 1000.0, "frequency": 0.5},
                {"label": "Call", "kind": "call", "to_mbb": 2000.0, "frequency": 0.4},
                {"label": "Raise 6", "kind": "raise", "to_mbb": 6000.0, "frequency": 0.1}
            ],
            "by_class": [{"label": "T8s", "row": 4, "col": 6, "weight": 4.0, "frequencies": [0.6, 0.3, 0.1]}],
            "ranges": [], "history": [], "strategy_note": null, "reach_probability": 1.0, "stack_to_pot": null
        }))
        .unwrap();
        let c: HandClass = "T8s".parse().unwrap();
        let h = preflop_frequencies(&node, c, Difficulty::Hard).unwrap();
        assert!(h.iter().zip([0.6, 0.3, 0.1]).all(|(a, b)| (a - b).abs() < 1e-12), "{h:?}");
        let m = preflop_frequencies(&node, c, Difficulty::Medium).unwrap();
        assert!((m[0] - 0.45).abs() < 1e-12 && (m[1] - 0.45).abs() < 1e-12);
        let e = preflop_frequencies(&node, c, Difficulty::Easy).unwrap();
        assert_eq!(e[2], 0.0, "easy bots re-raise only the top 5%");
        assert!(preflop_frequencies(&node, "AA".parse().unwrap(), Difficulty::Hard).is_none());
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let picks: Vec<usize> = (0..200).map(|_| sample(&[0.6, 0.3, 0.1], &mut rng)).collect();
        assert!(picks.iter().filter(|&&p| p == 0).count() > 90);
    }
}
