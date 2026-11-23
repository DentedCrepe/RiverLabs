//! Runs RiverLab [`SolveSpec`]s on GTOpen, the audited third-party postflop solver
//! (Apache-2.0, pinned commit [`GTOPEN_COMMIT`]; see `docs/solver-comparison.md`).
//!
//! Contract:
//! - Inputs are converted **exactly** (integer milli-big-blinds become `f64` chips, which is
//!   lossless). Anything GTOpen cannot represent is rejected with [`AdapterError::Unsupported`]
//!   rather than approximated. The only accepted approximation, rake rounding, is listed in
//!   [`SolveOutcome::approximations`] with its worst-case effect.
//! - OOP's "donk" sizes (leading into the previous street's aggressor) are set to the normal
//!   bet sizes. GTOpen's empty default would silently remove that option from the tree.
//! - Every outcome records the backend and commit so stored results stay traceable.
//! - Nothing here is shown to users until the Milestone 2 validation passes (ADR-0005).

use gtopen::tree::{Action, BetSize, KIND_ACTION, KIND_CHANCE, KIND_TERM_FOLD, StreetSizing, TreeConfig};
use gtopen::{Algorithm, Solver, Spot, SpotConfig};
use poker_core::{Card, Chips};
use range_engine::{Combo, Range};
use serde::{Deserialize, Serialize};
use solver_interface::{AveragingScheme, CfrVariant, SolveSpec};
use solver_validation::{Rake, RiverGame, RiverNode};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use thiserror::Error;

pub const BACKEND: &str = "gtopen";
pub const GTOPEN_REPOSITORY: &str = "https://github.com/MatthewPDingle/GTOpen";
pub const GTOPEN_COMMIT: &str = "eb5ccd671223293527f36d219cfe33baf0b0475c";

#[derive(Debug, Clone, PartialEq, Error)]
pub enum AdapterError {
    #[error("invalid solve specification: {0}")]
    InvalidSpec(String),
    #[error("not supported by the GTOpen backend: {0}")]
    Unsupported(String),
    #[error("GTOpen error: {0}")]
    Backend(String),
}

/// A GTOpen spot configuration derived from a [`SolveSpec`].
#[derive(Debug, Clone)]
pub struct Prepared {
    pub config: SpotConfig,
    pub algorithm: Algorithm,
    /// Live combos (not blocked by the board) and their weights, per player, OOP first.
    pub ranges: [Vec<(Combo, f64)>; 2],
    pub approximations: Vec<String>,
}

fn sizes(pcts: &[u32]) -> Vec<BetSize> {
    pcts.iter().map(|&p| BetSize::PotPct(p as f64)).collect()
}

/// Converts a validated [`SolveSpec`] into a GTOpen spot.
pub fn prepare(spec: &SolveSpec) -> Result<Prepared, AdapterError> {
    spec.validate().map_err(|e| AdapterError::InvalidSpec(e.to_string()))?;
    let unsupported = |m: String| Err(AdapterError::Unsupported(m));
    let game = &spec.game;
    let stake = game.stake.as_ref();
    let pot = Chips(spec.starting_pot_mbb);
    let mut approximations = Vec::new();

    // ---- ranges (validated by RiverLab's range engine) ----
    let dead = spec.board.card_set();
    let mut ranges: [Vec<(Combo, f64)>; 2] = [Vec::new(), Vec::new()];
    for (p, text) in spec.ranges.iter().enumerate() {
        let r = Range::parse(text).map_err(|e| AdapterError::InvalidSpec(format!("range {p}: {e}")))?;
        ranges[p] = r.without_cards(dead).iter().collect();
        if ranges[p].is_empty() {
            return Err(AdapterError::InvalidSpec(format!("range {p} is empty after removing board cards")));
        }
    }

    // ---- rake ----
    let rake = &game.rake;
    let to_chips = |a: poker_core::rake::RakeAmount, what: &str| {
        a.to_chips(stake, what).map_err(|e| AdapterError::InvalidSpec(e.to_string()))
    };
    let cap = rake.cap.map(|c| to_chips(c, "rake cap")).transpose()?;
    let mut rake_pct = rake.percentage_bps as f64 / 10_000.0;
    if cap == Some(Chips::ZERO) {
        rake_pct = 0.0; // a zero cap means no rake is ever taken
    }
    if let Some(min) = rake.min_pot.map(|m| to_chips(m, "minimum raked pot")).transpose()? {
        // Every postflop terminal pot is at least the starting pot, so a threshold at or below it
        // never binds and the model is exact. Above it, GTOpen cannot express the rule.
        if min > pot {
            return unsupported(format!("a minimum raked pot ({min}) above the starting pot ({pot})"));
        }
    }
    let increment = to_chips(rake.rounding.increment, "rake rounding increment")?;
    if rake_pct > 0.0 && increment > Chips(1) {
        approximations.push(format!(
            "Rake rounding to multiples of {increment} is not modelled; GTOpen takes the exact percentage. \
             Each terminal's payoff differs by less than {increment}."
        ));
    }
    if rake_pct > 0.0 && increment == Chips(1) {
        approximations.push(
            "Rake rounding to 0.001bb is not modelled; GTOpen takes the exact percentage (difference < 0.001bb per hand)."
                .into(),
        );
    }
    // No-flop-no-drop never applies postflop; heads-up with one effective stack has no side pots.

    // ---- tree ----
    let t = &spec.tree;
    let street = |s: usize, oop: bool| {
        let p = if oop { &t.oop } else { &t.ip };
        StreetSizing {
            bet: sizes(&p.bet_pct_pot[s]),
            raise: sizes(&p.raise_pct_pot[s]),
            donk: if oop { sizes(&p.bet_pct_pot[s]) } else { Vec::new() },
        }
    };
    let tree = TreeConfig {
        starting_pot: spec.starting_pot_mbb as f64,
        effective_stack: spec.effective_stack_mbb as f64,
        rake_pct,
        rake_cap: cap.map_or(0.0, |c| c.mbb() as f64),
        oop: [street(0, true), street(1, true), street(2, true)],
        ip: [street(0, false), street(1, false), street(2, false)],
        allin_threshold: t.all_in_threshold_pct_stack as f64 / 100.0,
        add_allin: t.add_all_in,
        max_raises: t.max_raises_per_street,
        carry_aggressor_through_checks: None,
        icm: None,
    };

    // ---- algorithm ----
    let s = &spec.solver;
    let algorithm = match (s.variant, s.averaging) {
        (CfrVariant::Discounted { alpha, beta, gamma }, AveragingScheme::Quadratic)
            if alpha == 1.5 && beta == 0.0 && gamma == 2.0 =>
        {
            Algorithm::Dcfr
        }
        (CfrVariant::CfrPlus, AveragingScheme::Quadratic) => Algorithm::CfrPlus,
        (v, a) => {
            return unsupported(format!(
                "{v:?} with {a:?} averaging (supported: Discounted CFR α=1.5 β=0 γ=2, or CFR+, with quadratic averaging)"
            ));
        }
    };
    if s.stopping.max_iterations > u32::MAX as u64 {
        return unsupported("more than 4,294,967,295 iterations".into());
    }

    let board: String = spec.board.cards().iter().map(|c| c.to_string()).collect();
    let fmt = |r: &[(Combo, f64)]| r.iter().map(|(c, w)| format!("{c}:{w}")).collect::<Vec<_>>().join(",");
    let config = SpotConfig { board, range_oop: fmt(&ranges[0]), range_ip: fmt(&ranges[1]), tree };
    Ok(Prepared { config, algorithm, ranges, approximations })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    TargetReached,
    MaxIterations,
    MaxSeconds,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub iterations: u64,
    /// Average gain available to a deviating player, in mbb per hand.
    pub exploitability_mbb: f64,
    pub exploitability_pct_pot: f64,
    pub elapsed_ms: u64,
}

/// Strategy at one decision node, action-major over the acting player's combos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeStrategy {
    pub player: usize,
    pub actions: Vec<String>,
    pub combos: Vec<String>,
    pub frequencies: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveOutcome {
    pub backend: String,
    pub backend_repository: String,
    pub backend_commit: String,
    pub algorithm: String,
    pub stop_reason: StopReason,
    pub progress: Progress,
    /// Expected value per hand in mbb, net of each player's half of the starting pot (OOP, IP).
    pub values_mbb: [f64; 2],
    pub approximations: Vec<String>,
    pub root: NodeStrategy,
}

/// A solved game, kept for inspection and export.
pub struct Solved {
    solver: Solver,
    prepared: Prepared,
    pub outcome: SolveOutcome,
}

impl std::fmt::Debug for Solved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Solved").field("outcome", &self.outcome).finish_non_exhaustive()
    }
}

fn action_label(a: &Action) -> String {
    let bb = |x: f64| Chips(x.round() as u64).to_string();
    match a {
        Action::Fold => "fold".into(),
        Action::Check => "check".into(),
        Action::Call(x) => format!("call {}", bb(*x)),
        Action::Bet(x) => format!("bet {}", bb(*x)),
        Action::Raise(x) => format!("raise to {}", bb(*x)),
    }
}

fn values(solver: &Solver) -> [f64; 2] {
    let spot = &*solver.spot;
    let z = solver.pair_weight_sum();
    std::array::from_fn(|p| {
        let cfv = solver.traverse_avg(0, p, &spot.weights[1 - p], gtopen::game::Dealt::default());
        spot.weights[p].iter().zip(&cfv).map(|(&w, &v)| w as f64 * v as f64).sum::<f64>() / z
    })
}

fn our_card(c: gtopen::cards::Card) -> Result<Card, AdapterError> {
    Card::from_str(&gtopen::cards::card_to_string(c)).map_err(|e| AdapterError::Backend(e.to_string()))
}

/// Solves a spec. `cancel` is checked every iteration; `on_progress` runs at every
/// exploitability check (every `exploitability_check_interval` iterations and at the end).
pub fn solve(
    spec: &SolveSpec,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(&Progress),
) -> Result<Solved, AdapterError> {
    let prepared = prepare(spec)?;
    let spot = Spot::new(prepared.config.clone()).map_err(AdapterError::Backend)?;
    check_hands(&spot, &prepared.ranges)?;
    let mut solver = Solver::new(Arc::new(spot));
    solver.algo = prepared.algorithm;

    let stop = &spec.solver.stopping;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(spec.solver.threads as usize)
        .build()
        .map_err(|e| AdapterError::Backend(e.to_string()))?;
    let start = Instant::now();
    let pot = spec.starting_pot_mbb as f64;
    let mut progress =
        Progress { iterations: 0, exploitability_mbb: f64::NAN, exploitability_pct_pot: f64::NAN, elapsed_ms: 0 };
    let reason = loop {
        if cancel.load(Ordering::Relaxed) {
            break StopReason::Cancelled;
        }
        pool.install(|| solver.iterate());
        let it = solver.iteration as u64;
        let last = it >= stop.max_iterations;
        let timed_out = stop.max_seconds.is_some_and(|s| start.elapsed().as_secs() >= s);
        if last || timed_out || it % stop.exploitability_check_interval == 0 {
            let e = pool.install(|| solver.exploitability());
            progress = Progress {
                iterations: it,
                exploitability_mbb: e,
                exploitability_pct_pot: e / pot * 100.0,
                elapsed_ms: start.elapsed().as_millis() as u64,
            };
            on_progress(&progress);
            if stop.target_exploitability_pct_pot.is_some_and(|t| progress.exploitability_pct_pot <= t) {
                break StopReason::TargetReached;
            }
            if last {
                break StopReason::MaxIterations;
            }
            if timed_out {
                break StopReason::MaxSeconds;
            }
        }
    };
    if !progress.exploitability_mbb.is_finite() && reason != StopReason::Cancelled {
        return Err(AdapterError::Backend("exploitability is not finite".into()));
    }
    solver.ensure_symmetric();
    let root = node_strategy(&solver, 0)?;
    let outcome = SolveOutcome {
        backend: BACKEND.into(),
        backend_repository: GTOPEN_REPOSITORY.into(),
        backend_commit: GTOPEN_COMMIT.into(),
        algorithm: prepared.algorithm.name().into(),
        stop_reason: reason,
        progress,
        values_mbb: values(&solver),
        approximations: prepared.approximations.clone(),
        root,
    };
    Ok(Solved { solver, prepared, outcome })
}

/// GTOpen must see exactly the combos and weights RiverLab parsed.
fn check_hands(spot: &Spot, ranges: &[Vec<(Combo, f64)>; 2]) -> Result<(), AdapterError> {
    for (p, ours) in ranges.iter().enumerate() {
        let mut theirs: Vec<(Combo, f64)> = Vec::new();
        for h in &spot.hands[p] {
            let c =
                Combo::new(our_card(h.c1)?, our_card(h.c2)?).ok_or_else(|| AdapterError::Backend("bad hand".into()))?;
            if h.weight > 0.0 {
                theirs.push((c, h.weight as f64));
            }
        }
        let mut ours = ours.clone();
        theirs.sort_by_key(|x| x.0);
        ours.sort_by_key(|x| x.0);
        let same =
            theirs.len() == ours.len() && theirs.iter().zip(&ours).all(|(a, b)| a.0 == b.0 && (a.1 - b.1).abs() < 1e-6);
        if !same {
            return Err(AdapterError::Backend(format!(
                "GTOpen parsed player {p}'s range differently ({} vs {} combos)",
                theirs.len(),
                ours.len()
            )));
        }
    }
    Ok(())
}

fn node_strategy(solver: &Solver, idx: u32) -> Result<NodeStrategy, AdapterError> {
    let tree = &solver.spot.tree;
    let node = &tree.nodes[idx as usize];
    if node.kind != KIND_ACTION {
        return Err(AdapterError::Backend(format!("node {idx} is not a decision node")));
    }
    let p = node.player as usize;
    let n_act = node.num_children as usize;
    let actions = (0..n_act).map(|a| action_label(&tree.actions[node.actions_start as usize + a])).collect();
    let combos = solver.spot.hands[p]
        .iter()
        .map(|h| Ok(Combo::new(our_card(h.c1)?, our_card(h.c2)?).expect("distinct").to_string()))
        .collect::<Result<_, AdapterError>>()?;
    let frequencies = solver.average_strategy(idx, node).into_iter().map(f64::from).collect();
    Ok(NodeStrategy { player: p, actions, combos, frequencies })
}

impl Solved {
    pub fn solver(&self) -> &Solver {
        &self.solver
    }

    /// Exports a solved **river** spot for independent checking with `solver-validation`.
    /// Ranges and weights come from RiverLab's own parse; only the tree shape, contributions and
    /// average strategies come from GTOpen.
    pub fn river_game(&self) -> Result<RiverGame, AdapterError> {
        let tree = &self.solver.spot.tree;
        if tree.root_street != 2 {
            return Err(AdapterError::Unsupported("export is implemented for river spots only".into()));
        }
        let board: [Card; 5] = Card::parse_many(&self.prepared.config.board)
            .map_err(|e| AdapterError::Backend(e.to_string()))?
            .try_into()
            .map_err(|_| AdapterError::Backend("river board must have 5 cards".into()))?;
        // GTOpen hand order -> index into RiverLab's range order.
        let mut ranges = self.prepared.ranges.clone();
        let index_maps: [Vec<usize>; 2] = std::array::from_fn(|p| {
            ranges[p].sort_by_key(|x| x.0);
            self.solver.spot.hands[p]
                .iter()
                .map(|h| {
                    let c = Combo::new(our_card(h.c1).unwrap(), our_card(h.c2).unwrap()).unwrap();
                    ranges[p].binary_search_by_key(&c, |x| x.0).expect("checked by check_hands")
                })
                .collect()
        });
        let mut nodes = Vec::new();
        self.export(0, &index_maps, &ranges, &mut nodes)?;
        let cfg = &self.prepared.config.tree;
        let rake = Rake { fraction: cfg.rake_pct, cap: (cfg.rake_cap > 0.0).then_some(cfg.rake_cap) };
        Ok(RiverGame { board, ranges, rake, nodes })
    }

    fn export(
        &self,
        idx: u32,
        maps: &[Vec<usize>; 2],
        ranges: &[Vec<(Combo, f64)>; 2],
        out: &mut Vec<RiverNode>,
    ) -> Result<usize, AdapterError> {
        let tree = &self.solver.spot.tree;
        let node = &tree.nodes[idx as usize];
        let me = out.len();
        match node.kind {
            KIND_ACTION => {
                out.push(RiverNode::Showdown { put: [0.0; 2] }); // placeholder, replaced below
                let p = node.player as usize;
                let n_act = node.num_children as usize;
                let theirs = self.solver.average_strategy(idx, node);
                let nh = maps[p].len();
                let n = ranges[p].len();
                let mut strategy = vec![0.0; n_act * n];
                for a in 0..n_act {
                    for (i, &ours) in maps[p].iter().enumerate() {
                        strategy[a * n + ours] = theirs[a * nh + i] as f64;
                    }
                }
                let mut children = Vec::with_capacity(n_act);
                for a in 0..n_act {
                    let child = tree.children[node.children_start as usize + a];
                    children.push(self.export(child, maps, ranges, out)?);
                }
                out[me] = RiverNode::Decision { player: p, children, strategy };
            }
            KIND_TERM_FOLD => out.push(RiverNode::Fold { folder: node.player as usize, put: node.put }),
            KIND_CHANCE => return Err(AdapterError::Unsupported("chance node in a river export".into())),
            _ => out.push(RiverNode::Showdown { put: node.put }),
        }
        Ok(me)
    }
}
