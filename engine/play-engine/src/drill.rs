//! Postflop drills from a saved solve: deal a decision, grade the answer by EV.
//!
//! A drill is dealt by playing the solved game forward: a hand is drawn for each player from
//! their starting range (by weight, without shared cards), each decision follows the solver's
//! strategy for that player's hand, and turn and river cards are dealt at random from the cards
//! left. One of your decisions on the way is the drill, so spots come up as often as they would
//! when both players follow the solution.
//!
//! The answer is graded by the EV of your action for your exact hand against the opponent's
//! range at that point (both players following the solution afterwards), relative to the best
//! action. Thresholds are in `docs/trainer.md`.

use crate::config::{PlayError, Result};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use range_engine::Combo;
use serde::{Deserialize, Serialize};
use solver_gtopen::{PathStep, Solved};
use solver_interface::SolveSpec;

/// EV losses up to this share of the starting pot count as correct, up to `CLOSE_PCT_POT` as close.
pub const CORRECT_PCT_POT: f64 = 1.0;
pub const CLOSE_PCT_POT: f64 = 5.0;
/// At each of your decisions on the way, the chance the deal stops there.
const STOP_CHANCE: f64 = 0.5;
const MAX_TRIES: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryStep {
    /// 0 = out of position, 1 = in position; `None` for a dealt card.
    pub player: Option<u8>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostflopDrill {
    /// 0 = out of position, 1 = in position.
    pub hero_player: u8,
    pub hero_combo: String,
    pub hero_class: String,
    pub path: Vec<PathStep>,
    pub history: Vec<HistoryStep>,
    pub street: String,
    pub board: Vec<String>,
    /// Chips in the middle when this street began (mbb).
    pub pot_mbb: f64,
    /// Each player's chips put in on this street so far (mbb), [oop, ip].
    pub street_bets_mbb: [f64; 2],
    /// Each player's chips behind (mbb), [oop, ip].
    pub stacks_mbb: [f64; 2],
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostflopGrade {
    pub actions: Vec<String>,
    /// The solver's mix for your exact hand.
    pub strategy: Vec<f64>,
    /// EV of each action for your hand (bb, from the start of the solve, net of your half of its
    /// starting pot).
    pub action_evs_bb: Vec<f64>,
    pub taken: usize,
    pub best: usize,
    pub ev_loss_bb: f64,
    /// EV loss as a percentage of the solve's starting pot.
    pub ev_loss_pct_pot: f64,
    /// "correct", "close" or "mistake".
    pub verdict: String,
}

fn combo_of(s: &str) -> Result<Combo> {
    Ok(Combo::from(
        s.parse::<poker_core::HoleCards>().map_err(|_| PlayError::InvalidConfig(format!("invalid hand {s}")))?,
    ))
}

fn pick<T: Copy>(items: &[(T, f64)], rng: &mut ChaCha8Rng) -> Option<T> {
    let total: f64 = items.iter().map(|(_, w)| w).sum();
    if total <= 0.0 {
        return None;
    }
    let mut x = rng.random::<f64>() * total;
    for (item, w) in items {
        x -= w;
        if x <= 0.0 {
            return Some(*item);
        }
    }
    items.last().map(|(i, _)| *i)
}

/// Deals a drill. `hero`: 0 (out of position), 1 (in position) or `None` for either.
pub fn deal(solved: &Solved, spec: &SolveSpec, seed: u64, hero: Option<u8>) -> Result<PostflopDrill> {
    let e = |x: solver_gtopen::AdapterError| PlayError::Engine(x.to_string());
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let any = solved.hand_at(&[], 0, Combo::all().next().expect("combos")).map_err(e)?;
    for _ in 0..MAX_TRIES {
        let hp = hero.unwrap_or_else(|| u8::from(rng.random::<bool>())) as usize;
        let Some(mine) = pick(&any.ranges[hp], &mut rng) else { break };
        let theirs: Vec<(Combo, f64)> =
            any.ranges[1 - hp].iter().filter(|(c, _)| !c.card_set().intersects(mine.card_set())).copied().collect();
        let Some(other) = pick(&theirs, &mut rng) else { continue };
        let combos = if hp == 0 { [mine, other] } else { [other, mine] };
        let mut path: Vec<PathStep> = Vec::new();
        let mut history = Vec::new();
        let mut street_start: Vec<PathStep> = Vec::new();
        loop {
            let view = solved.node_view(&path).map_err(e)?;
            match view.kind.as_str() {
                "decision" => {
                    let p = view.player.expect("decision player");
                    let node = solved.hand_at(&path, p, combos[p]).map_err(e)?;
                    let Some(sigma) = node.strategy.clone() else { break };
                    if p == hp && node.action_evs_mbb.is_some() && rng.random::<f64>() < STOP_CHANCE {
                        let start = solved.node_view(&street_start).map_err(e)?;
                        let half = spec.starting_pot_mbb as f64 / 2.0;
                        let pot = start.put_mbb[0] + start.put_mbb[1];
                        return Ok(PostflopDrill {
                            hero_player: hp as u8,
                            hero_combo: mine.to_string(),
                            hero_class: range_engine::HandClass::of(mine).to_string(),
                            path,
                            history,
                            street: view.street,
                            board: view.board,
                            pot_mbb: pot,
                            street_bets_mbb: [view.put_mbb[0] - start.put_mbb[0], view.put_mbb[1] - start.put_mbb[1]],
                            stacks_mbb: [
                                spec.effective_stack_mbb as f64 - (view.put_mbb[0] - half),
                                spec.effective_stack_mbb as f64 - (view.put_mbb[1] - half),
                            ],
                            actions: node.actions,
                        });
                    }
                    let options: Vec<(usize, f64)> = sigma.iter().copied().enumerate().collect();
                    let Some(a) = pick(&options, &mut rng) else { break };
                    history.push(HistoryStep { player: Some(p as u8), label: node.actions[a].clone() });
                    path.push(PathStep::Action(a));
                }
                "chance" => {
                    let cards: Vec<poker_core::Card> = view
                        .cards
                        .iter()
                        .filter_map(|c| c.parse().ok())
                        .filter(|c: &poker_core::Card| !combos.iter().any(|x| x.card_set().contains(*c)))
                        .collect();
                    if cards.is_empty() {
                        break;
                    }
                    let c = cards[rng.random_range(0..cards.len())];
                    history.push(HistoryStep { player: None, label: c.to_string() });
                    path.push(PathStep::Card(c));
                    street_start = path.clone();
                }
                _ => break, // the hand ended before a drill was chosen: deal again
            }
        }
    }
    Err(PlayError::State("could not deal a decision from this solve (the chosen player may never get to act)".into()))
}

/// Grades `choice` (an action index) for the drill's hand.
pub fn grade(solved: &Solved, spec: &SolveSpec, drill: &PostflopDrill, choice: usize) -> Result<PostflopGrade> {
    let e = |x: solver_gtopen::AdapterError| PlayError::Engine(x.to_string());
    let combo = combo_of(&drill.hero_combo)?;
    let node = solved.hand_at(&drill.path, drill.hero_player as usize, combo).map_err(e)?;
    let (Some(strategy), Some(evs)) = (node.strategy, node.action_evs_mbb) else {
        return Err(PlayError::State("this drill is not your decision with this hand".into()));
    };
    if choice >= evs.len() {
        return Err(PlayError::IllegalAction(format!("there are only {} actions", evs.len())));
    }
    let best = (0..evs.len()).max_by(|&i, &j| evs[i].total_cmp(&evs[j])).expect("at least one action");
    let loss_mbb = (evs[best] - evs[choice]).max(0.0);
    let pct = 100.0 * loss_mbb / spec.starting_pot_mbb as f64;
    let verdict = if pct <= CORRECT_PCT_POT {
        "correct"
    } else if pct <= CLOSE_PCT_POT {
        "close"
    } else {
        "mistake"
    };
    Ok(PostflopGrade {
        actions: node.actions,
        strategy,
        action_evs_bb: evs.iter().map(|x| x / 1000.0).collect(),
        taken: choice,
        best,
        ev_loss_bb: loss_mbb / 1000.0,
        ev_loss_pct_pot: pct,
        verdict: verdict.into(),
    })
}
