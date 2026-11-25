//! Push/fold trainer: heads-up jam-or-fold drills graded against the exact equilibrium.
//!
//! The reference is `solver_validation::push_fold`: every one of the 1,326 combos with full card
//! removal, exact all-in equities (all 1,712,304 boards per pair), solved with CFR+ and checked by
//! exact best response (`docs/solver-validation.md`). Blinds 0.5/1, equal stacks, **no ante, no
//! rake**: the only preflop scope RiverLab has validated.
//!
//! A drill deals you a hand at a stack depth in the small blind (jam or fold) or the big blind
//! facing a jam (call or fold). The grade compares the expected value of your action with the
//! best action against the opponent's equilibrium strategy. Thresholds are in `docs/trainer.md`.

use crate::config::{PlayError, Result};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use range_engine::{COMBO_COUNT, Combo, HandClass};
use serde::{Deserialize, Serialize};
use solver_gtopen::ClassStrategy;
use solver_validation::push_fold::{ComboEquities, ComboEvs, PushFold, PushFoldEvaluation, combo_evs, evaluate, solve};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Smallest and largest stack (mbb) a drill can use, and the step between depths.
pub const MIN_STACK_MBB: u64 = 2_000;
pub const MAX_STACK_MBB: u64 = 25_000;
pub const STACK_STEP_MBB: u64 = 500;
/// CFR+ iterations per stack depth (exploitability is measured and reported with every result).
pub const ITERATIONS: usize = 1_000;
/// Actions whose EVs differ by at most this many bb count as equally good.
pub const EQUAL_BB: f64 = 0.001;
/// A wrong action costing at most this many bb is "close" rather than a mistake.
pub const CLOSE_BB: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Seat {
    /// Small blind (the button heads-up): jam or fold.
    Sb,
    /// Big blind facing the small blind's jam: call or fold.
    Bb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    Fold,
    /// Jam (small blind) or call the jam (big blind).
    Play,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Drill {
    pub stack_mbb: u64,
    pub seat: Seat,
    pub cards: Vec<String>,
    pub class: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grade {
    pub drill: Drill,
    pub choice: Choice,
    /// ["fold", "jam"] or ["fold", "call"].
    pub actions: Vec<String>,
    /// The equilibrium's frequency of each action with this exact combo.
    pub frequencies: [f64; 2],
    /// Expected value (bb, from the start of the hand, blinds included) of each action against
    /// the opponent's equilibrium strategy.
    pub evs_bb: [f64; 2],
    /// EV of the best action minus EV of yours (bb).
    pub ev_loss_bb: f64,
    /// "correct" (within EQUAL_BB of the best), "close" (within CLOSE_BB) or "mistake".
    pub verdict: String,
    /// Exploitability of the reference solution at this depth (bb per hand).
    pub exploitability_bb: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chart {
    pub stack_mbb: u64,
    /// Small blind per class: frequencies [fold, jam]; weight = combos in the class.
    pub sb: Vec<ClassStrategy>,
    /// Big blind facing a jam per class: frequencies [fold, call].
    pub bb: Vec<ClassStrategy>,
    pub sb_jam_share: f64,
    pub bb_call_share: f64,
    /// Small blind's EV per hand at equilibrium (bb); the big blind's is the negative.
    pub sb_ev_bb: f64,
    pub exploitability_bb: f64,
    pub iterations: usize,
}

struct Solved {
    profile: PushFold,
    evs: ComboEvs,
    eval: PushFoldEvaluation,
}

/// Exact solutions per stack depth, computed on first use and kept in memory. Dealing a drill
/// starts its stack's solution in the background (about 2 s on 4 cores), so it is usually ready
/// by the time the answer is graded.
pub struct PushFoldTrainer {
    inner: Arc<Inner>,
}

struct Inner {
    eq: Arc<ComboEquities>,
    solved: Mutex<HashMap<u64, Arc<Solved>>>,
    /// One lock per stack depth, held while it is solved, so no depth is solved twice at once.
    solving: Mutex<HashMap<u64, Arc<Mutex<()>>>>,
}

impl Inner {
    fn solution(&self, stack_mbb: u64) -> Result<Arc<Solved>> {
        check_stack(stack_mbb)?;
        let lock =
            |m: &Mutex<HashMap<u64, Arc<Solved>>>| m.lock().unwrap_or_else(|e| e.into_inner()).get(&stack_mbb).cloned();
        if let Some(s) = lock(&self.solved) {
            return Ok(s);
        }
        let gate = self.solving.lock().unwrap_or_else(|e| e.into_inner()).entry(stack_mbb).or_default().clone();
        let _solving = gate.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(s) = lock(&self.solved) {
            return Ok(s); // solved by someone else while this waited
        }
        let stack = stack_mbb as f64 / 1000.0;
        let profile = solve(&self.eq, stack, ITERATIONS);
        let eval = evaluate(&self.eq, stack, &profile);
        let evs = combo_evs(&self.eq, stack, &profile);
        let s = Arc::new(Solved { profile, evs, eval });
        self.solved.lock().unwrap_or_else(|e| e.into_inner()).insert(stack_mbb, s.clone());
        Ok(s)
    }
}

impl std::fmt::Debug for PushFoldTrainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PushFoldTrainer").finish_non_exhaustive()
    }
}

pub fn check_stack(stack_mbb: u64) -> Result<()> {
    if !(MIN_STACK_MBB..=MAX_STACK_MBB).contains(&stack_mbb) || stack_mbb % STACK_STEP_MBB != 0 {
        return Err(PlayError::InvalidConfig(format!(
            "push/fold stacks are {}bb to {}bb in steps of {}bb",
            MIN_STACK_MBB / 1000,
            MAX_STACK_MBB / 1000,
            STACK_STEP_MBB as f64 / 1000.0
        )));
    }
    Ok(())
}

fn parse_combo(cards: &[String]) -> Result<Combo> {
    let bad = || PlayError::InvalidConfig(format!("invalid hand {cards:?}"));
    if cards.len() != 2 {
        return Err(bad());
    }
    let a = cards[0].parse().map_err(|_| bad())?;
    let b = cards[1].parse().map_err(|_| bad())?;
    Combo::new(a, b).ok_or_else(bad)
}

/// Class-level view of a combo-level frequency (combos weighted equally).
fn by_class(freq: &[f64]) -> (Vec<ClassStrategy>, f64) {
    let mut acc: HashMap<HandClass, (f64, f64)> = HashMap::new();
    for c in Combo::all() {
        let e = acc.entry(HandClass::of(c)).or_default();
        e.0 += freq[c.index()];
        e.1 += 1.0;
    }
    let mut out: Vec<ClassStrategy> = acc
        .into_iter()
        .map(|(class, (sum, n))| {
            let (row, col) = class.grid_position();
            let f = sum / n;
            ClassStrategy { label: class.to_string(), row, col, weight: n, frequencies: vec![1.0 - f, f] }
        })
        .collect();
    out.sort_by_key(|c| (c.row, c.col));
    (out, freq.iter().sum::<f64>() / COMBO_COUNT as f64)
}

impl PushFoldTrainer {
    pub fn new(eq: Arc<ComboEquities>) -> PushFoldTrainer {
        PushFoldTrainer {
            inner: Arc::new(Inner { eq, solved: Mutex::new(HashMap::new()), solving: Mutex::new(HashMap::new()) }),
        }
    }

    fn solution(&self, stack_mbb: u64) -> Result<Arc<Solved>> {
        self.inner.solution(stack_mbb)
    }

    /// Starts solving `stack_mbb` on a background thread (no-op if it is solved or in progress).
    pub fn prepare_in_background(&self, stack_mbb: u64) {
        if self.inner.solved.lock().unwrap_or_else(|e| e.into_inner()).contains_key(&stack_mbb) {
            return;
        }
        let inner = self.inner.clone();
        std::thread::spawn(move || {
            let _ = inner.solution(stack_mbb);
        });
    }

    /// Deals a drill: a stack depth in [min, max] (in steps), a seat (random if `None`) and a hand.
    pub fn drill(&self, min_mbb: u64, max_mbb: u64, seat: Option<Seat>, seed: u64) -> Result<Drill> {
        check_stack(min_mbb)?;
        check_stack(max_mbb)?;
        if min_mbb > max_mbb {
            return Err(PlayError::InvalidConfig("the smallest stack is above the largest".into()));
        }
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let steps = (max_mbb - min_mbb) / STACK_STEP_MBB;
        let stack_mbb = min_mbb + rng.random_range(0..=steps) * STACK_STEP_MBB;
        let seat = seat.unwrap_or(if rng.random::<bool>() { Seat::Sb } else { Seat::Bb });
        let combo = Combo::all().nth(rng.random_range(0..COMBO_COUNT)).expect("index in range");
        let (a, b) = combo.cards();
        // Solve this depth while the player thinks, so grading rarely waits.
        self.prepare_in_background(stack_mbb);
        Ok(Drill {
            stack_mbb,
            seat,
            cards: vec![a.to_string(), b.to_string()],
            class: HandClass::of(combo).to_string(),
        })
    }

    pub fn grade(&self, drill: &Drill, choice: Choice) -> Result<Grade> {
        let combo = parse_combo(&drill.cards)?;
        let s = self.solution(drill.stack_mbb)?;
        let i = combo.index();
        let (actions, play_freq, play_ev, fold_ev) = match drill.seat {
            Seat::Sb => (["fold", "jam"], s.profile.jam[i], s.evs.jam[i], -0.5),
            Seat::Bb => (["fold", "call"], s.profile.call[i], s.evs.call[i], -1.0),
        };
        if play_ev.is_nan() {
            return Err(PlayError::Engine("the small blind never jams against this hand at this depth".into()));
        }
        let evs = [fold_ev, play_ev];
        let mine = evs[usize::from(choice == Choice::Play)];
        let loss = (evs[0].max(evs[1]) - mine).max(0.0);
        let verdict = if loss <= EQUAL_BB {
            "correct"
        } else if loss <= CLOSE_BB {
            "close"
        } else {
            "mistake"
        };
        Ok(Grade {
            drill: Drill { class: HandClass::of(combo).to_string(), ..drill.clone() },
            choice,
            actions: actions.iter().map(|a| a.to_string()).collect(),
            frequencies: [1.0 - play_freq, play_freq],
            evs_bb: evs,
            ev_loss_bb: loss,
            verdict: verdict.into(),
            exploitability_bb: s.eval.exploitability,
        })
    }

    /// The equilibrium at one depth, by hand class.
    pub fn chart(&self, stack_mbb: u64) -> Result<Chart> {
        let s = self.solution(stack_mbb)?;
        let (sb, sb_jam_share) = by_class(&s.profile.jam);
        let (bb, bb_call_share) = by_class(&s.profile.call);
        Ok(Chart {
            stack_mbb,
            sb,
            bb,
            sb_jam_share,
            bb_call_share,
            sb_ev_bb: s.eval.sb_ev,
            exploitability_bb: s.eval.exploitability,
            iterations: ITERATIONS,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Equities from a toy rule (higher top card wins), enough to exercise grading quickly.
    fn toy() -> PushFoldTrainer {
        let eq = ComboEquities::from_fn(|a, b| {
            let top = |c: Combo| c.cards().0.rank().index().max(c.cards().1.rank().index());
            match top(a).cmp(&top(b)) {
                std::cmp::Ordering::Greater => 0.7,
                std::cmp::Ordering::Less => 0.3,
                std::cmp::Ordering::Equal => 0.5,
            }
        });
        PushFoldTrainer::new(Arc::new(eq))
    }

    #[test]
    fn drills_are_seeded_and_within_bounds() {
        let t = toy();
        let d = t.drill(5_000, 8_000, None, 3).unwrap();
        assert_eq!(d, t.drill(5_000, 8_000, None, 3).unwrap());
        assert!((5_000..=8_000).contains(&d.stack_mbb) && d.stack_mbb % STACK_STEP_MBB == 0);
        assert!(t.drill(5_000, 8_100, None, 3).is_err());
        assert!(t.drill(9_000, 8_000, None, 3).is_err());
        assert_eq!(t.drill(5_000, 5_000, Some(Seat::Bb), 9).unwrap().seat, Seat::Bb);
    }

    #[test]
    fn grades_compare_evs_against_the_equilibrium() {
        let t = toy();
        let drill = |cards: [&str; 2], seat| Drill {
            stack_mbb: 10_000,
            seat,
            cards: cards.iter().map(|c| c.to_string()).collect(),
            class: String::new(),
        };
        // An ace is the strongest top card here: jamming is right, folding loses EV.
        let jam = t.grade(&drill(["As", "2d"], Seat::Sb), Choice::Play).unwrap();
        assert_eq!(jam.verdict, "correct");
        assert_eq!(jam.drill.class, "A2o");
        assert!(jam.frequencies[1] > 0.99 && jam.ev_loss_bb == 0.0);
        let fold = t.grade(&drill(["As", "2d"], Seat::Sb), Choice::Fold).unwrap();
        assert_eq!(fold.verdict, "mistake");
        assert!((fold.ev_loss_bb - (fold.evs_bb[1] - fold.evs_bb[0])).abs() < 1e-12);
        assert_eq!(fold.evs_bb[0], -0.5);
        // Deuce-high calling a jam is a mistake for the big blind; folding is worth −1.
        let call = t.grade(&drill(["2s", "3d"], Seat::Bb), Choice::Play).unwrap();
        assert_eq!(call.verdict, "mistake");
        assert_eq!(call.evs_bb[0], -1.0);
        assert!(t.grade(&drill(["As", "As"], Seat::Sb), Choice::Play).is_err());
        let chart = t.chart(10_000).unwrap();
        assert_eq!(chart.sb.len(), 169);
        assert_eq!(chart.sb.iter().map(|c| c.weight).sum::<f64>(), 1326.0);
        assert!(chart.exploitability_bb < 1e-3, "{}", chart.exploitability_bb);
    }

    /// With the real exact table (built by `scripts/build-push-fold-table.sh`), the 10bb
    /// solution matches the figures in docs/solver-validation.md.
    #[test]
    fn real_table_matches_the_documented_validation() {
        let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../target/hu_combo_equity.bin"));
        let Some(eq) = ComboEquities::load(path) else {
            eprintln!("skipped: {} not built", path.display());
            return;
        };
        let t = PushFoldTrainer::new(Arc::new(eq));
        let c = t.chart(10_000).unwrap();
        assert!((c.sb_jam_share - 0.583).abs() < 0.002, "{}", c.sb_jam_share);
        assert!((c.bb_call_share - 0.374).abs() < 0.002, "{}", c.bb_call_share);
        assert!((c.sb_ev_bb + 0.04537).abs() < 1e-4, "{}", c.sb_ev_bb);
        assert!(c.exploitability_bb < 1e-5);
        let aa =
            Drill { stack_mbb: 10_000, seat: Seat::Bb, cards: vec!["Ah".into(), "Ad".into()], class: String::new() };
        assert_eq!(t.grade(&aa, Choice::Play).unwrap().verdict, "correct");
        assert_eq!(t.grade(&aa, Choice::Fold).unwrap().verdict, "mistake");
    }
}
