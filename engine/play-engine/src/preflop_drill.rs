//! Preflop drills dealt from a saved GTOpen preflop solve (docs/trainer.md, "Preflop drills").
//!
//! GTOpen's preflop solver is a **modelled approximation** (flop pots are valued by a model, not
//! solved), and it exposes each hand class's strategy but not the value of each action. So a
//! drill is graded by how often *the model* takes your action with your hand class, never by EV:
//!
//! - `matches`: your action is the one the model takes most often with this hand;
//! - `mixed`: not the most frequent, but the model takes it at least [`MIXED_MIN`] of the time;
//! - `differs`: the model takes it less often than that.
//!
//! These thresholds are RiverLab's choice and are documented with the drill.

use crate::config::{PlayError, Result};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use range_engine::HandClass;
use serde::{Deserialize, Serialize};
use solver_gtopen::ClassStrategy;
use solver_gtopen::preflop::{PreflopBrowser, PreflopNode, PreflopStep};
use std::str::FromStr;

/// A frequency at or above this, for an action that is not the model's most frequent one, is
/// graded `mixed` rather than `differs`.
pub const MIXED_MIN: f64 = 0.15;
/// In "any" spots, the chance of stopping at each decision on the way down the tree.
const ANY_STOP: f64 = 0.35;
const ATTEMPTS: usize = 400;

/// Anything that can show nodes of a solved preflop tree.
pub trait PreflopTree {
    fn node(&self, path: &[usize]) -> std::result::Result<PreflopNode, String>;
}

impl PreflopTree for PreflopBrowser {
    fn node(&self, path: &[usize]) -> std::result::Result<PreflopNode, String> {
        PreflopBrowser::node(self, path).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflopSpot {
    /// Everyone before you folded: open or fold (or limp, when the solve allows it).
    Open,
    /// One player raised first in and everyone between folded: fold, call or re-raise.
    FacingRaise,
    /// Any decision, reached by following the model's own frequencies.
    Any,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrillAction {
    pub label: String,
    pub kind: String,
    pub to_mbb: f64,
}

/// A dealt decision. It carries no strategy: the answer is only revealed by [`grade`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreflopDrill {
    pub spot: PreflopSpot,
    /// Action indices from the root to this decision.
    pub path: Vec<usize>,
    pub position: String,
    pub class: String,
    pub cards: [String; 2],
    pub positions: Vec<String>,
    pub pot_mbb: f64,
    pub invested_mbb: Vec<f64>,
    pub live: Vec<bool>,
    pub actions: Vec<DrillAction>,
    /// What happened before you: each earlier decision and the action taken.
    pub history: Vec<PreflopStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradedAction {
    pub label: String,
    pub kind: String,
    /// How often the model takes this action with your hand class (0..1).
    pub frequency: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreflopGrade {
    pub drill: PreflopDrill,
    pub choice: usize,
    pub choice_label: String,
    /// "matches", "mixed" or "differs" (see the module docs).
    pub verdict: String,
    pub your_frequency: f64,
    /// The action the model takes most often with this hand.
    pub model_action: usize,
    pub actions: Vec<GradedAction>,
    /// The model's strategy at this decision for every class in the range (for a chart).
    pub by_class: Vec<ClassStrategy>,
    pub mixed_min: f64,
}

fn engine(e: String) -> PlayError {
    PlayError::Engine(e)
}

fn is_raise(kind: &str) -> bool {
    kind != "fold" && kind != "call" && kind != "check"
}

/// Index of an action of `kind` that is always available ("fold"), if there is one.
fn fold_index(node: &PreflopNode) -> Option<usize> {
    node.actions.iter().position(|a| a.kind == "fold")
}

/// Picks an index with probability proportional to `weights` (None if they sum to zero).
fn pick(rng: &mut ChaCha8Rng, weights: &[f64]) -> Option<usize> {
    let total: f64 = weights.iter().filter(|w| **w > 0.0).sum();
    if total <= 0.0 {
        return None;
    }
    let mut x = rng.random::<f64>() * total;
    for (i, w) in weights.iter().enumerate() {
        if *w <= 0.0 {
            continue;
        }
        if x < *w {
            return Some(i);
        }
        x -= w;
    }
    weights.iter().rposition(|w| *w > 0.0)
}

fn decision(node: &PreflopNode) -> bool {
    node.kind == "action" && node.strategy_note.is_none() && !node.by_class.is_empty()
}

/// Follows folds from `path` until `target` acts. None if the hand ends or a fold is missing.
fn fold_to(tree: &dyn PreflopTree, mut path: Vec<usize>, target: &str) -> Result<Option<(Vec<usize>, PreflopNode)>> {
    for _ in 0..12 {
        let node = tree.node(&path).map_err(engine)?;
        if node.kind != "action" {
            return Ok(None);
        }
        if node.actor.as_deref() == Some(target) {
            return Ok(Some((path, node)));
        }
        match fold_index(&node) {
            Some(f) => path.push(f),
            None => return Ok(None),
        }
    }
    Ok(None)
}

fn try_spot(
    tree: &dyn PreflopTree,
    spot: PreflopSpot,
    rng: &mut ChaCha8Rng,
) -> Result<Option<(Vec<usize>, PreflopNode)>> {
    let root = tree.node(&[]).map_err(engine)?;
    let positions = root.positions.clone();
    let big_blind = positions.last().cloned().unwrap_or_default();
    match spot {
        PreflopSpot::Open => {
            // Anyone but the big blind can be first in.
            let openers: Vec<&String> = positions.iter().filter(|p| **p != big_blind).collect();
            let target = openers[rng.random_range(0..openers.len())].clone();
            fold_to(tree, Vec::new(), &target)
        }
        PreflopSpot::FacingRaise => {
            let openers: Vec<&String> = positions.iter().filter(|p| **p != big_blind).collect();
            let opener = openers[rng.random_range(0..openers.len())].clone();
            let Some((mut path, node)) = fold_to(tree, Vec::new(), &opener)? else { return Ok(None) };
            let weights: Vec<f64> =
                node.actions.iter().map(|a| if is_raise(&a.kind) { a.frequency } else { 0.0 }).collect();
            let Some(raise) = pick(rng, &weights) else { return Ok(None) };
            path.push(raise);
            let after: Vec<&String> = positions.iter().skip_while(|p| **p != opener).skip(1).collect();
            if after.is_empty() {
                return Ok(None);
            }
            let hero = after[rng.random_range(0..after.len())].clone();
            fold_to(tree, path, &hero)
        }
        PreflopSpot::Any => {
            let mut path = Vec::new();
            for _ in 0..24 {
                let node = tree.node(&path).map_err(engine)?;
                if node.kind != "action" {
                    return Ok(None);
                }
                if decision(&node) && rng.random::<f64>() < ANY_STOP {
                    return Ok(Some((path, node)));
                }
                let weights: Vec<f64> = node.actions.iter().map(|a| a.frequency).collect();
                let Some(a) = pick(rng, &weights) else { return Ok(None) };
                path.push(a);
            }
            Ok(None)
        }
    }
}

/// Deals a decision of the given kind, and a hand for it drawn from the model's range there
/// (each combo in the range equally likely).
pub fn deal(tree: &dyn PreflopTree, spot: PreflopSpot, seed: u64) -> Result<PreflopDrill> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    for _ in 0..ATTEMPTS {
        let Some((path, node)) = try_spot(tree, spot, &mut rng)? else { continue };
        if !decision(&node) {
            continue;
        }
        let weights: Vec<f64> = node.by_class.iter().map(|c| c.weight).collect();
        let Some(i) = pick(&mut rng, &weights) else { continue };
        let label = node.by_class[i].label.clone();
        let class = HandClass::from_str(&label).map_err(|e| engine(e.to_string()))?;
        let combos = class.combos();
        let (a, b) = combos[rng.random_range(0..combos.len())].cards();
        return Ok(PreflopDrill {
            spot,
            path,
            position: node.actor.clone().unwrap_or_default(),
            class: label,
            cards: [a.to_string(), b.to_string()],
            positions: node.positions.clone(),
            pot_mbb: node.pot_mbb,
            invested_mbb: node.invested_mbb.clone(),
            live: node.live.clone(),
            actions: node
                .actions
                .iter()
                .map(|a| DrillAction { label: a.label.clone(), kind: a.kind.clone(), to_mbb: a.to_mbb })
                .collect(),
            history: node.history.iter().filter(|h| h.chosen.is_some()).cloned().collect(),
        });
    }
    Err(PlayError::State(format!(
        "this solve has no reachable {} decision to drill",
        match spot {
            PreflopSpot::Open => "open-or-fold",
            PreflopSpot::FacingRaise => "facing-a-raise",
            PreflopSpot::Any => "",
        }
    )))
}

/// Grades `choice` (an action index) against the model's strategy for the drill's hand class.
pub fn grade(tree: &dyn PreflopTree, drill: &PreflopDrill, choice: usize) -> Result<PreflopGrade> {
    let node = tree.node(&drill.path).map_err(engine)?;
    if !decision(&node) || node.actor.as_deref() != Some(drill.position.as_str()) {
        return Err(PlayError::InvalidConfig("the drill does not match this solve".into()));
    }
    if choice >= node.actions.len() {
        return Err(PlayError::IllegalAction(format!("no action {choice} here")));
    }
    let class = node
        .by_class
        .iter()
        .find(|c| c.label == drill.class && c.weight > 0.0)
        .ok_or_else(|| PlayError::InvalidConfig(format!("{} is not in the range at this decision", drill.class)))?;
    let freqs = &class.frequencies;
    let model_action = (0..freqs.len()).max_by(|&a, &b| freqs[a].total_cmp(&freqs[b])).expect("a decision has actions");
    let your = freqs[choice];
    let verdict = if your >= freqs[model_action] - 1e-9 {
        "matches"
    } else if your >= MIXED_MIN {
        "mixed"
    } else {
        "differs"
    };
    Ok(PreflopGrade {
        drill: drill.clone(),
        choice,
        choice_label: node.actions[choice].label.clone(),
        verdict: verdict.into(),
        your_frequency: your,
        model_action,
        actions: node
            .actions
            .iter()
            .zip(freqs)
            .map(|(a, f)| GradedAction { label: a.label.clone(), kind: a.kind.clone(), frequency: *f })
            .collect(),
        by_class: node.by_class.clone(),
        mixed_min: MIXED_MIN,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solver_gtopen::preflop::PreflopAction;

    /// A three-handed toy tree: BTN folds or raises; SB then BB act; strategies are fixed
    /// test inputs (not poker data).
    struct Toy;

    fn class(label: &str, freqs: &[f64]) -> ClassStrategy {
        let c = HandClass::from_str(label).unwrap();
        let (row, col) = c.grid_position();
        ClassStrategy { label: label.into(), row, col, weight: c.combo_count() as f64, frequencies: freqs.to_vec() }
    }

    fn act(label: &str, kind: &str, freq: f64) -> PreflopAction {
        PreflopAction { label: label.into(), kind: kind.into(), to_mbb: 0.0, frequency: freq }
    }

    fn node(actor: &str, actions: Vec<PreflopAction>, classes: Vec<ClassStrategy>, depth: usize) -> PreflopNode {
        PreflopNode {
            kind: "action".into(),
            actor: Some(actor.into()),
            positions: vec!["BTN".into(), "SB".into(), "BB".into()],
            pot_mbb: 1500.0,
            invested_mbb: vec![0.0, 500.0, 1000.0],
            live: vec![true; 3],
            actions,
            by_class: classes,
            ranges: vec![],
            history: (0..depth)
                .map(|i| PreflopStep { actor: format!("p{i}"), actions: vec![], chosen: Some(0) })
                .collect(),
            strategy_note: None,
            reach_probability: 1.0,
            stack_to_pot: None,
        }
    }

    fn terminal() -> PreflopNode {
        PreflopNode {
            kind: "fold_win".into(),
            actor: None,
            actions: vec![],
            by_class: vec![],
            ..node("BTN", vec![], vec![], 0)
        }
    }

    impl PreflopTree for Toy {
        fn node(&self, path: &[usize]) -> std::result::Result<PreflopNode, String> {
            let two = || vec![act("fold", "fold", 0.5), act("raise 2.5", "raise", 0.5)];
            let facing = || vec![act("fold", "fold", 0.6), act("call", "call", 0.3), act("raise 9", "raise", 0.1)];
            let classes = |f: &[f64]| vec![class("AA", f), class("72o", f)];
            Ok(match path {
                [] => node("BTN", two(), vec![class("AA", &[0.0, 1.0]), class("72o", &[1.0, 0.0])], 0),
                [0] => node("SB", two(), classes(&[0.3, 0.7]), 1),
                [0, 0] => terminal(),
                [0, 1] => node("BB", facing(), classes(&[0.2, 0.7, 0.1]), 2),
                [1] => node("SB", facing(), classes(&[0.6, 0.3, 0.1]), 1),
                [1, 0] => node("BB", facing(), classes(&[0.6, 0.3, 0.1]), 2),
                [1, _] | [0, 1, _] | [1, 0, _] => terminal(),
                _ => return Err("bad path".into()),
            })
        }
    }

    #[test]
    fn open_spots_are_reached_by_folds_and_hide_the_answer() {
        for seed in 0..40 {
            let d = deal(&Toy, PreflopSpot::Open, seed).unwrap();
            assert!(d.path.iter().all(|&a| a == 0), "only folds before an open: {:?}", d.path);
            assert_ne!(d.position, "BB");
            assert!(["AA", "72o"].contains(&d.class.as_str()));
            let c = HandClass::of(
                range_engine::Combo::new(d.cards[0].parse().unwrap(), d.cards[1].parse().unwrap()).unwrap(),
            );
            assert_eq!(c.to_string(), d.class, "the cards belong to the dealt class");
        }
        assert_eq!(deal(&Toy, PreflopSpot::Open, 7).unwrap(), deal(&Toy, PreflopSpot::Open, 7).unwrap(), "seeded");
    }

    #[test]
    fn facing_raise_spots_follow_one_raise_then_folds() {
        for seed in 0..40 {
            let d = deal(&Toy, PreflopSpot::FacingRaise, seed).unwrap();
            assert!(d.path == vec![1] || d.path == vec![1, 0] || d.path == vec![0, 1], "{:?}", d.path);
            assert_eq!(d.actions.len(), 3);
        }
        assert!(deal(&Toy, PreflopSpot::Any, 3).is_ok());
    }

    #[test]
    fn grading_compares_your_action_with_the_models_frequency_for_your_hand() {
        let mut d = deal(&Toy, PreflopSpot::Open, 1).unwrap();
        d.path = vec![];
        d.position = "BTN".into();
        d.class = "AA".into();
        let g = grade(&Toy, &d, 1).unwrap();
        assert_eq!((g.verdict.as_str(), g.model_action, g.your_frequency), ("matches", 1, 1.0));
        assert_eq!(grade(&Toy, &d, 0).unwrap().verdict, "differs");
        // At BB facing a raise, 72o calls 70%, folds 20%, raises 10%.
        d.path = vec![0, 1];
        d.position = "BB".into();
        d.class = "72o".into();
        assert_eq!(grade(&Toy, &d, 1).unwrap().verdict, "matches");
        assert_eq!(grade(&Toy, &d, 0).unwrap().verdict, "mixed");
        assert_eq!(grade(&Toy, &d, 2).unwrap().verdict, "differs");
        assert!(grade(&Toy, &d, 9).is_err());
        d.class = "KK".into();
        assert!(grade(&Toy, &d, 0).is_err(), "a class outside the range is refused");
        d.class = "72o".into();
        d.position = "SB".into();
        assert!(grade(&Toy, &d, 0).is_err(), "a drill that does not match the tree is refused");
    }
}
