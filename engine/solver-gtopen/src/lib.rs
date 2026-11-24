//! Runs RiverLab [`SolveSpec`]s on GTOpen, the audited third-party postflop solver
//! (Apache-2.0, commit [`GTOPEN_COMMIT`] with RiverLab's [`GTOPEN_PATCHES`], vendored in
//! `third_party/gtopen`; see `docs/solver-comparison.md` and `docs/solver-validation.md`).
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
use solver_validation::{GameNode, PostflopGame, Rake};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use thiserror::Error;

pub mod preflop;

pub const BACKEND: &str = "gtopen";
pub const GTOPEN_REPOSITORY: &str = "https://github.com/MatthewPDingle/GTOpen";
/// The upstream GTOpen commit RiverLab's copy is taken from.
pub const GTOPEN_COMMIT: &str = "eb5ccd671223293527f36d219cfe33baf0b0475c";
/// RiverLab's changes to that commit, applied in `third_party/gtopen` (docs/solver-validation.md).
pub const GTOPEN_PATCHES: &[&str] = &["arenabuf-provenance"];
/// What results record as the solver that produced them: the commit plus RiverLab's patches.
pub const GTOPEN_BUILD: &str = "eb5ccd671223293527f36d219cfe33baf0b0475c+arenabuf-provenance";

#[derive(Debug, Clone, PartialEq, Error)]
pub enum AdapterError {
    #[error("invalid solve specification: {0}")]
    InvalidSpec(String),
    #[error("not supported by the GTOpen backend: {0}")]
    Unsupported(String),
    #[error("GTOpen error: {0}")]
    Backend(String),
    #[error(
        "spot too large: needs {} of memory, the limit is {}; use fewer bet sizes or narrower ranges",
        human_bytes(*needed_bytes),
        human_bytes(*limit_bytes)
    )]
    TooLarge { needed_bytes: u64, limit_bytes: u64 },
}

fn human_bytes(b: u64) -> String {
    match b {
        b if b >= 1 << 30 => format!("{:.2} GB", b as f64 / (1u64 << 30) as f64),
        b if b >= 1 << 20 => format!("{:.1} MB", b as f64 / (1u64 << 20) as f64),
        b if b >= 1 << 10 => format!("{:.1} KB", b as f64 / 1024.0),
        b => format!("{b} bytes"),
    }
}

/// Resource limits applied before any solver memory is allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Tree construction aborts past this many nodes.
    pub max_nodes: usize,
    /// Tree plus solver arenas.
    pub max_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_nodes: 50_000_000, max_bytes: 2 << 30 }
    }
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
    /// True while a node-locked solve is still solving the unlocked game it locks from.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unlocked_phase: bool,
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

/// The root decision aggregated per hand class, weighted by range weight over live combos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassStrategy {
    pub label: String,
    pub row: usize,
    pub col: usize,
    /// Sum of range weights of the class's live combos.
    pub weight: f64,
    /// Weighted average frequency of each root action (aligned with `NodeStrategy::actions`).
    pub frequencies: Vec<f64>,
}

/// RiverLab's own recomputation of the result (see `solver-validation`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndependentCheck {
    pub performed: bool,
    pub values_mbb: Option<[f64; 2]>,
    pub exploitability_mbb: Option<f64>,
    pub note: String,
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
    pub root_by_class: Vec<ClassStrategy>,
    /// Tree plus solver arenas, in bytes.
    pub memory_bytes: u64,
    pub independent_check: IndependentCheck,
    /// For a node-locked solve: the unlocked solve it was locked from (the equilibrium to compare
    /// the exploit with). `None` for an ordinary solve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlocked: Option<UnlockedBase>,
}

/// The unlocked solve a node-locked solve starts from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlockedBase {
    pub iterations: u64,
    pub exploitability_mbb: f64,
    pub exploitability_pct_pot: f64,
    pub values_mbb: [f64; 2],
}

/// Runs CFR iterations until a stopping rule fires, reporting progress at each check.
fn run(
    solver: &mut Solver,
    pool: &rayon::ThreadPool,
    stop: &solver_interface::StoppingCriteria,
    pot: f64,
    unlocked_phase: bool,
    cancel: &AtomicBool,
    on_progress: &mut impl FnMut(&Progress),
) -> (StopReason, Progress) {
    let start = Instant::now();
    let mut progress = Progress {
        unlocked_phase,
        iterations: 0,
        exploitability_mbb: f64::NAN,
        exploitability_pct_pot: f64::NAN,
        elapsed_ms: 0,
    };
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
                unlocked_phase,
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
    (reason, progress)
}

/// Turns the solved strategy at each locked decision into one with the requested range-wide
/// frequencies (GTOpen's range lock: every hand's frequencies are scaled from the solved
/// strategy, so hands keep their relative preferences).
fn apply_locks(solver: &mut Solver, spec: &SolveSpec) -> Result<(), AdapterError> {
    for (i, lock) in spec.locks.iter().enumerate() {
        let path: Vec<gtopen::PathStep> = lock.path.iter().map(|&index| gtopen::PathStep::Action { index }).collect();
        let walk = solver.walk_path(&path).map_err(|e| AdapterError::InvalidSpec(format!("lock {i}: {e}")))?;
        let node = &solver.spot.tree.nodes[walk.node_idx as usize];
        if node.kind != KIND_ACTION {
            return Err(AdapterError::InvalidSpec(format!("lock {i}: the path does not end at a decision")));
        }
        if node.num_children as usize != lock.frequencies.len() {
            return Err(AdapterError::InvalidSpec(format!(
                "lock {i}: that decision has {} actions, but {} frequencies were given",
                node.num_children,
                lock.frequencies.len()
            )));
        }
        let freqs = lock.frequencies.iter().map(|&f| f as f32).collect();
        solver
            .lock_node(&path, gtopen::LockMode::Range { freqs }, format!("lock {i}"))
            .map_err(|e| AdapterError::InvalidSpec(format!("lock {i}: {e}")))?;
    }
    Ok(())
}

/// A solved game, kept for inspection and export.
pub struct Solved {
    solver: Solver,
    prepared: Prepared,
    limits: Limits,
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
    on_progress: impl FnMut(&Progress),
) -> Result<Solved, AdapterError> {
    solve_with_limits(spec, Limits::default(), cancel, on_progress)
}

/// The memory a solve of `spec` needs (the tree is built, no solver memory is allocated).
pub fn memory_needed(spec: &SolveSpec, max_nodes: usize) -> Result<u64, AdapterError> {
    let prepared = prepare(spec)?;
    let spot = Spot::new_with_limit(prepared.config, Some(max_nodes)).map_err(AdapterError::Backend)?;
    let ds = spot.tree.data_size;
    Ok(8 * (ds[0] + ds[1]) + spot.tree_bytes())
}

/// Like [`solve`], refusing spots that exceed `limits` before allocating solver memory.
pub fn solve_with_limits(
    spec: &SolveSpec,
    limits: Limits,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(&Progress),
) -> Result<Solved, AdapterError> {
    let prepared = prepare(spec)?;
    let spot = Spot::new_with_limit(prepared.config.clone(), Some(limits.max_nodes)).map_err(|e| {
        if e.starts_with("tree too large") {
            AdapterError::Backend(format!("{e} (limit {} nodes)", limits.max_nodes))
        } else {
            AdapterError::Backend(e)
        }
    })?;
    let ds = spot.tree.data_size;
    let memory_bytes = 8 * (ds[0] + ds[1]) + spot.tree_bytes();
    if memory_bytes > limits.max_bytes {
        return Err(AdapterError::TooLarge { needed_bytes: memory_bytes, limit_bytes: limits.max_bytes });
    }
    check_hands(&spot, &prepared.ranges)?;
    let mut solver = Solver::new(Arc::new(spot));
    solver.algo = prepared.algorithm;

    let stop = &spec.solver.stopping;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(spec.solver.threads as usize)
        .build()
        .map_err(|e| AdapterError::Backend(e.to_string()))?;
    let pot = spec.starting_pot_mbb as f64;
    let locked = !spec.locks.is_empty();
    let (mut reason, mut progress) = run(&mut solver, &pool, stop, pot, locked, cancel, &mut on_progress);
    let mut unlocked = None;
    let mut approximations = prepared.approximations.clone();
    if locked && reason != StopReason::Cancelled {
        if !progress.exploitability_mbb.is_finite() {
            return Err(AdapterError::Backend("exploitability is not finite".into()));
        }
        unlocked = Some(UnlockedBase {
            iterations: progress.iterations,
            exploitability_mbb: progress.exploitability_mbb,
            exploitability_pct_pot: progress.exploitability_pct_pot,
            values_mbb: values(&solver),
        });
        // Lock from the solved strategy, then solve again from scratch with only the locks
        // carried over, so the averages contain no pre-lock iterations.
        apply_locks(&mut solver, spec)?;
        let locks = std::mem::take(&mut solver.locks);
        let labels = std::mem::take(&mut solver.lock_labels);
        let spot = solver.spot.clone();
        drop(solver);
        solver = Solver::new(spot);
        solver.algo = prepared.algorithm;
        solver.locks = locks;
        solver.lock_labels = labels;
        (reason, progress) = run(&mut solver, &pool, stop, pot, false, cancel, &mut on_progress);
        approximations.push(format!(
            "Node-locked: {} decision(s) are fixed at the frequencies you set. The other decisions are \
             solved as a response to them, so this is an exploit of those locks, not an equilibrium. \
             Exploitability counts only what a player could gain at decisions that are not locked.",
            spec.locks.len()
        ));
    }
    if !progress.exploitability_mbb.is_finite() && reason != StopReason::Cancelled {
        return Err(AdapterError::Backend("exploitability is not finite".into()));
    }
    solver.ensure_symmetric();
    let root = node_strategy(&solver, 0)?;
    let root_by_class = by_class(&root, &prepared.ranges)?;
    let outcome = SolveOutcome {
        backend: BACKEND.into(),
        backend_repository: GTOPEN_REPOSITORY.into(),
        backend_commit: GTOPEN_BUILD.into(),
        algorithm: prepared.algorithm.name().into(),
        stop_reason: reason,
        progress,
        values_mbb: values(&solver),
        approximations,
        root,
        root_by_class,
        memory_bytes,
        independent_check: IndependentCheck {
            performed: false,
            values_mbb: None,
            exploitability_mbb: None,
            note: "not run".into(),
        },
        unlocked,
    };
    let mut solved = Solved { solver, prepared, limits, outcome };
    solved.outcome.independent_check = solved.independent_check();
    Ok(solved)
}

fn by_class(root: &NodeStrategy, ranges: &[Vec<(Combo, f64)>; 2]) -> Result<Vec<ClassStrategy>, AdapterError> {
    use range_engine::HandClass;
    let weight: std::collections::HashMap<Combo, f64> = ranges[root.player].iter().copied().collect();
    let n = root.combos.len();
    let k = root.actions.len();
    let mut acc: std::collections::BTreeMap<(usize, usize), (String, f64, Vec<f64>)> = Default::default();
    for (i, label) in root.combos.iter().enumerate() {
        let hc: poker_core::HoleCards =
            label.parse().map_err(|e: poker_core::PokerError| AdapterError::Backend(e.to_string()))?;
        let combo = Combo::from(hc);
        let w = *weight.get(&combo).unwrap_or(&0.0);
        let class = HandClass::of(combo);
        let e = acc.entry(class.grid_position()).or_insert_with(|| (class.to_string(), 0.0, vec![0.0; k]));
        e.1 += w;
        for a in 0..k {
            e.2[a] += w * root.frequencies[a * n + i];
        }
    }
    Ok(acc
        .into_iter()
        .filter(|(_, (_, w, _))| *w > 0.0)
        .map(|((row, col), (label, w, f))| ClassStrategy {
            label,
            row,
            col,
            weight: w,
            frequencies: f.into_iter().map(|x| x / w).collect(),
        })
        .collect())
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

/// One step from a node to a child: an action index at a decision node, or a dealt card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum PathStep {
    Action(usize),
    Card(Card),
}

/// What the UI needs to show one node of a solved tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeView {
    /// "decision", "chance", "fold" or "showdown".
    pub kind: String,
    pub street: String,
    pub board: Vec<String>,
    /// Chips committed by each player (each includes half the starting pot), in mbb.
    pub put_mbb: [f64; 2],
    /// Acting player at a decision node, folder at a fold.
    pub player: Option<usize>,
    pub actions: Vec<String>,
    /// Cards that can be dealt at a chance node.
    pub cards: Vec<String>,
    /// The acting player's strategy per hand class, weighted by that player's reach here.
    pub by_class: Vec<ClassStrategy>,
    /// Each player's reaching range mass as a fraction of their starting range mass.
    pub reach_fraction: [f64; 2],
}

struct Walk {
    idx: u32,
    combos: [Vec<Combo>; 2],
    reach: [Vec<f64>; 2],
    start: [f64; 2],
    board: Vec<Card>,
    dealt: gtopen::game::Dealt,
}

/// One hand at one node of a solved tree (see [`Solved::hand_at`]).
#[derive(Debug, Clone, PartialEq)]
pub struct HandAtNode {
    /// "decision", "chance", "fold" or "showdown".
    pub kind: String,
    pub player: Option<usize>,
    pub actions: Vec<String>,
    /// At a decision of the hand's player: the solver's average strategy for this exact hand.
    pub strategy: Option<Vec<f64>>,
    /// At a decision of the hand's player: the expected value (mbb) of each action for this hand,
    /// against the opponent's range as it reaches this node, with both players following the
    /// solved strategy afterwards. Measured from the start of the solved street.
    pub action_evs_mbb: Option<Vec<f64>>,
    /// Each player's range here: every hand with its reach (range weight times that player's own
    /// action probabilities along the path; hands blocked by dealt cards are dropped).
    pub ranges: [Vec<(Combo, f64)>; 2],
    pub reach_fraction: [f64; 2],
    pub board: Vec<String>,
}

/// Loads a solve saved by [`Solved::save`], re-deriving the spot from the same spec. The spec
/// must be the one the file was solved from (checked against the saved spot).
pub fn load(spec: &SolveSpec, path: &str) -> Result<Solved, AdapterError> {
    let prepared = prepare(spec)?;
    let mut solver = Solver::load(path).map_err(AdapterError::Backend)?;
    if solver.spot.config.board.to_lowercase() != prepared.config.board.to_lowercase() {
        return Err(AdapterError::Backend("saved solve does not match the spec (board)".into()));
    }
    check_hands(&solver.spot, &prepared.ranges)?;
    solver.ensure_symmetric();
    let ds = solver.spot.tree.data_size;
    let memory_bytes = 8 * (ds[0] + ds[1]) + solver.spot.tree_bytes();
    let root = node_strategy(&solver, 0)?;
    let root_by_class = by_class(&root, &prepared.ranges)?;
    let outcome = SolveOutcome {
        backend: BACKEND.into(),
        backend_repository: GTOPEN_REPOSITORY.into(),
        backend_commit: GTOPEN_BUILD.into(),
        algorithm: prepared.algorithm.name().into(),
        stop_reason: StopReason::MaxIterations,
        progress: Progress {
            unlocked_phase: false,
            iterations: solver.iteration as u64,
            exploitability_mbb: f64::NAN,
            exploitability_pct_pot: f64::NAN,
            elapsed_ms: 0,
        },
        values_mbb: values(&solver),
        approximations: prepared.approximations.clone(),
        root,
        root_by_class,
        memory_bytes,
        independent_check: IndependentCheck {
            performed: false,
            values_mbb: None,
            exploitability_mbb: None,
            note: "loaded from storage".into(),
        },
        unlocked: None,
    };
    Ok(Solved { solver, prepared, limits: Limits { max_nodes: usize::MAX, max_bytes: u64::MAX }, outcome })
}

impl Solved {
    pub fn solver(&self) -> &Solver {
        &self.solver
    }

    /// Writes the solved spot (GTOpen's format: JSON header + f32 arenas) to `path`.
    pub fn save(&mut self, path: &str) -> Result<(), AdapterError> {
        self.solver.save(path).map_err(AdapterError::Backend)
    }

    /// Follows `path` from the root: the node reached, each player's reach per hand (their range
    /// weight times their own action probabilities, zero for hands blocked by dealt cards), and
    /// the board and cards dealt on the way.
    fn walk(&self, path: &[PathStep]) -> Result<Walk, AdapterError> {
        let tree = &self.solver.spot.tree;
        let hands = &self.solver.spot.hands;
        let combos: [Vec<Combo>; 2] = std::array::from_fn(|p| {
            hands[p].iter().map(|h| Combo::new(our_card(h.c1).unwrap(), our_card(h.c2).unwrap()).unwrap()).collect()
        });
        let weight_of: [std::collections::HashMap<Combo, f64>; 2] =
            std::array::from_fn(|p| self.prepared.ranges[p].iter().copied().collect());
        let mut reach: [Vec<f64>; 2] =
            std::array::from_fn(|p| combos[p].iter().map(|c| *weight_of[p].get(c).unwrap_or(&0.0)).collect());
        let start: [f64; 2] = std::array::from_fn(|p| reach[p].iter().sum());
        let mut board =
            Card::parse_many(&self.prepared.config.board).map_err(|e| AdapterError::Backend(e.to_string()))?;
        let mut dealt = gtopen::game::Dealt::default();
        let bad = |m: String| Err(AdapterError::InvalidSpec(m));
        let mut idx = 0u32;
        for (k, step) in path.iter().enumerate() {
            let node = &tree.nodes[idx as usize];
            match (node.kind, step) {
                (KIND_ACTION, PathStep::Action(a)) if *a < node.num_children as usize => {
                    let p = node.player as usize;
                    let sigma = self.solver.average_strategy(idx, node);
                    let nh = hands[p].len();
                    for (i, r) in reach[p].iter_mut().enumerate() {
                        *r *= sigma[a * nh + i] as f64;
                    }
                    idx = tree.children[node.children_start as usize + a];
                }
                (KIND_CHANCE, PathStep::Card(c)) => {
                    if board.contains(c) {
                        return bad(format!("step {k}: {c} is already on the board"));
                    }
                    let gt = (0..52u8)
                        .find(|&g| our_card(g).is_ok_and(|x| x == *c))
                        .ok_or_else(|| AdapterError::Backend("card mapping".into()))?;
                    let child = tree.children[node.children_start as usize + gt as usize];
                    if child == u32::MAX {
                        return bad(format!("step {k}: {c} cannot be dealt here"));
                    }
                    for p in 0..2 {
                        for (i, r) in reach[p].iter_mut().enumerate() {
                            if combos[p][i].card_set().contains(*c) {
                                *r = 0.0;
                            }
                        }
                    }
                    board.push(*c);
                    dealt = dealt.push(gt);
                    idx = child;
                }
                _ => return bad(format!("step {k} ({step:?}) does not match the node there")),
            }
        }
        Ok(Walk { idx, combos, reach, start, board, dealt })
    }

    /// The node reached from the root by `path`, with reach-weighted class strategies.
    pub fn node_view(&self, path: &[PathStep]) -> Result<NodeView, AdapterError> {
        let tree = &self.solver.spot.tree;
        let hands = &self.solver.spot.hands;
        let Walk { idx, combos, reach, start, board, .. } = self.walk(path)?;
        let node = &tree.nodes[idx as usize];
        let street = match board.len() {
            3 => "flop",
            4 => "turn",
            _ => "river",
        };
        let mut view = NodeView {
            kind: String::new(),
            street: street.into(),
            board: board.iter().map(|c| c.to_string()).collect(),
            put_mbb: node.put,
            player: None,
            actions: Vec::new(),
            cards: Vec::new(),
            by_class: Vec::new(),
            reach_fraction: std::array::from_fn(|p| {
                if start[p] > 0.0 { reach[p].iter().sum::<f64>() / start[p] } else { 0.0 }
            }),
        };
        match node.kind {
            KIND_ACTION => {
                let p = node.player as usize;
                let n_act = node.num_children as usize;
                view.kind = "decision".into();
                view.player = Some(p);
                view.actions =
                    (0..n_act).map(|a| action_label(&tree.actions[node.actions_start as usize + a])).collect();
                let sigma = self.solver.average_strategy(idx, node);
                let nh = hands[p].len();
                let mut acc: std::collections::BTreeMap<(usize, usize), (String, f64, Vec<f64>)> = Default::default();
                for i in 0..nh {
                    let w = reach[p][i];
                    if w <= 0.0 {
                        continue;
                    }
                    let class = range_engine::HandClass::of(combos[p][i]);
                    let e =
                        acc.entry(class.grid_position()).or_insert_with(|| (class.to_string(), 0.0, vec![0.0; n_act]));
                    e.1 += w;
                    for a in 0..n_act {
                        e.2[a] += w * sigma[a * nh + i] as f64;
                    }
                }
                view.by_class = acc
                    .into_iter()
                    .map(|((row, col), (label, w, f))| ClassStrategy {
                        label,
                        row,
                        col,
                        weight: w,
                        frequencies: f.into_iter().map(|x| x / w).collect(),
                    })
                    .collect();
            }
            KIND_CHANCE => {
                view.kind = "chance".into();
                for g in 0..52u8 {
                    if tree.children[node.children_start as usize + g as usize] != u32::MAX {
                        let c = our_card(g)?;
                        if !board.contains(&c) {
                            view.cards.push(c.to_string());
                        }
                    }
                }
            }
            KIND_TERM_FOLD => {
                view.kind = "fold".into();
                view.player = Some(node.player as usize);
            }
            _ => view.kind = "showdown".into(),
        }
        Ok(view)
    }

    /// `combo` (held by `player`) at the node reached by `path`: its strategy and the EV of each
    /// action there (when it is that player's decision), and both players' ranges.
    pub fn hand_at(&self, path: &[PathStep], player: usize, combo: Combo) -> Result<HandAtNode, AdapterError> {
        if player > 1 {
            return Err(AdapterError::InvalidSpec("player must be 0 (out of position) or 1".into()));
        }
        let tree = &self.solver.spot.tree;
        let w = self.walk(path)?;
        let node = &tree.nodes[w.idx as usize];
        let kind = match node.kind {
            KIND_ACTION => "decision",
            KIND_CHANCE => "chance",
            KIND_TERM_FOLD => "fold",
            _ => "showdown",
        };
        let mut out = HandAtNode {
            kind: kind.into(),
            player: (node.kind == KIND_ACTION || node.kind == KIND_TERM_FOLD).then_some(node.player as usize),
            actions: Vec::new(),
            strategy: None,
            action_evs_mbb: None,
            ranges: std::array::from_fn(|p| {
                w.combos[p].iter().zip(&w.reach[p]).filter(|(_, r)| **r > 0.0).map(|(c, r)| (*c, *r)).collect()
            }),
            reach_fraction: std::array::from_fn(|p| {
                if w.start[p] > 0.0 { w.reach[p].iter().sum::<f64>() / w.start[p] } else { 0.0 }
            }),
            board: w.board.iter().map(|c| c.to_string()).collect(),
        };
        if node.kind != KIND_ACTION {
            return Ok(out);
        }
        let n_act = node.num_children as usize;
        out.actions = (0..n_act).map(|a| action_label(&tree.actions[node.actions_start as usize + a])).collect();
        if node.player as usize != player {
            return Ok(out);
        }
        let Some(i) = w.combos[player].iter().position(|c| *c == combo) else {
            return Ok(out); // not in this player's range at all
        };
        let nh = w.combos[player].len();
        let sigma = self.solver.average_strategy(w.idx, node);
        out.strategy = Some((0..n_act).map(|a| sigma[a * nh + i] as f64).collect());
        // Counterfactual values below each child, normalised by the opponent reach that does not
        // collide with this hand: the hand's expected value for each action.
        let opp = 1 - player;
        let reach_o: Vec<f32> = w.reach[opp].iter().map(|&r| r as f32).collect();
        let norm: f64 = w.combos[opp]
            .iter()
            .zip(&w.reach[opp])
            .filter(|(c, _)| !c.card_set().intersects(combo.card_set()))
            .map(|(_, r)| *r)
            .sum();
        if norm > 0.0 {
            let evs = (0..n_act)
                .map(|a| {
                    let child = tree.children[node.children_start as usize + a];
                    self.solver.traverse_avg(child, player, &reach_o, w.dealt)[i] as f64 / norm
                })
                .collect();
            out.action_evs_mbb = Some(evs);
        }
        Ok(out)
    }

    /// Recomputes values and exploitability with RiverLab's own validator, if the exported tree
    /// (about the size of the solver's own arenas) fits in the memory left under the limit.
    pub fn independent_check(&self) -> IndependentCheck {
        let skipped =
            |note: String| IndependentCheck { performed: false, values_mbb: None, exploitability_mbb: None, note };
        let ds = self.solver.spot.tree.data_size;
        let export_bytes = 8 * (ds[0] + ds[1]);
        if self.outcome.memory_bytes.saturating_add(export_bytes) > self.limits.max_bytes {
            return skipped(format!(
                "Skipped: the independent check needs about {} more memory than the {} limit allows. \
                 GTOpen's own exploitability is reported.",
                human_bytes(export_bytes),
                human_bytes(self.limits.max_bytes)
            ));
        }
        match self
            .postflop_game()
            .map_err(|e| e.to_string())
            .and_then(|g| solver_validation::evaluate(&g).map_err(|e| e.to_string()))
        {
            Ok(e) => IndependentCheck {
                performed: true,
                values_mbb: Some(e.ev),
                exploitability_mbb: Some(e.exploitability),
                note: "Recomputed by RiverLab's independent validator.".into(),
            },
            Err(err) => skipped(format!("Independent check failed: {err}")),
        }
    }

    /// Exports the solved game (river, turn or flop root) for independent checking with
    /// `solver-validation`. Ranges and weights come from RiverLab's own parse; only the tree
    /// shape, contributions and average strategies come from GTOpen. After `ensure_symmetric`,
    /// every dealt card has its own fully populated subtree, so suit-isomorphic runouts are
    /// checked too.
    pub fn postflop_game(&self) -> Result<PostflopGame, AdapterError> {
        let board = Card::parse_many(&self.prepared.config.board).map_err(|e| AdapterError::Backend(e.to_string()))?;
        let mut ranges = self.prepared.ranges.clone();
        for r in ranges.iter_mut() {
            r.sort_by_key(|x| x.0);
        }
        // GTOpen hand order -> index into RiverLab's (sorted) range order.
        let mut maps: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for (p, map) in maps.iter_mut().enumerate() {
            for h in &self.solver.spot.hands[p] {
                let c = Combo::new(our_card(h.c1)?, our_card(h.c2)?)
                    .ok_or_else(|| AdapterError::Backend("bad hand".into()))?;
                let i = ranges[p]
                    .binary_search_by_key(&c, |x| x.0)
                    .map_err(|_| AdapterError::Backend(format!("unknown combo {c}")))?;
                map.push(i);
            }
        }
        let mut nodes = Vec::new();
        let mut locked = std::collections::BTreeSet::new();
        let board_mask = board.iter().fold(0u64, |m, c| m | 1 << c.index());
        self.export(0, &maps, &ranges, board_mask, &mut nodes, &mut locked)?;
        let cfg = &self.prepared.config.tree;
        let rake = Rake { fraction: cfg.rake_pct, cap: (cfg.rake_cap > 0.0).then_some(cfg.rake_cap) };
        Ok(PostflopGame { board, ranges, rake, nodes, locked })
    }

    fn export(
        &self,
        idx: u32,
        maps: &[Vec<usize>; 2],
        ranges: &[Vec<(Combo, f64)>; 2],
        board: u64,
        out: &mut Vec<GameNode>,
        locked: &mut std::collections::BTreeSet<usize>,
    ) -> Result<usize, AdapterError> {
        let tree = &self.solver.spot.tree;
        let node = &tree.nodes[idx as usize];
        let me = out.len();
        out.push(GameNode::Fold { folder: 0, put: [0.0; 2] }); // placeholder, replaced below
        let filled = match node.kind {
            KIND_ACTION => {
                if self.solver.locks.contains_key(&idx) {
                    locked.insert(me);
                }
                let p = node.player as usize;
                let n_act = node.num_children as usize;
                let theirs = self.solver.average_strategy(idx, node);
                let nh = maps[p].len();
                let n = ranges[p].len();
                let mut strategy = vec![1.0 / n_act as f64; n_act * n];
                for (i, &ours) in maps[p].iter().enumerate() {
                    // Rows of combos blocked by dealt cards are unreachable; keep them uniform.
                    if ranges[p][ours].0.card_set().bits() & board != 0 {
                        continue;
                    }
                    for a in 0..n_act {
                        strategy[a * n + ours] = theirs[a * nh + i] as f64;
                    }
                }
                let mut children = Vec::with_capacity(n_act);
                for a in 0..n_act {
                    let child = tree.children[node.children_start as usize + a];
                    children.push(self.export(child, maps, ranges, board, out, locked)?);
                }
                GameNode::Decision { player: p, children, strategy }
            }
            KIND_CHANCE => {
                let mut children = Vec::new();
                for card in 0..52u8 {
                    let child = tree.children[node.children_start as usize + card as usize];
                    if board & (1u64 << card) != 0 {
                        continue;
                    }
                    let ours = Card::from_str(&gtopen::cards::card_to_string(card))
                        .map_err(|e| AdapterError::Backend(e.to_string()))?;
                    let bit = 1u64 << ours.index();
                    children.push((ours, self.export(child, maps, ranges, board | bit, out, locked)?));
                }
                GameNode::Chance { children }
            }
            KIND_TERM_FOLD => GameNode::Fold { folder: node.player as usize, put: node.put },
            _ => GameNode::Showdown { put: node.put },
        };
        out[me] = filled;
        Ok(me)
    }
}

/// GTOpen's heads-up push/fold preflop strategy at class level (169 hand classes), for
/// validation against RiverLab's exact combo-level solution.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassPushFold {
    /// Class label (e.g. "AKs") → small blind's jam frequency.
    pub jam: std::collections::HashMap<String, f64>,
    /// Class label → big blind's call frequency facing the jam.
    pub call: std::collections::HashMap<String, f64>,
}

/// Runs GTOpen's preflop solver on heads-up jam/fold at `stack` bb (blinds 0.5/1, no ante, no
/// rake), with its Monte Carlo class equity table of `equity_samples` per class pair.
/// `card_removal` toggles GTOpen's blocker model.
pub fn gtopen_hu_push_fold(
    stack: f64,
    card_removal: bool,
    equity_samples: u32,
    iterations: usize,
) -> Result<ClassPushFold, AdapterError> {
    use gtopen::preflop::equity::{EquityTable, NUM_CLASSES, class_label};
    use gtopen::preflop::{PreflopConfig, PreflopSolver};
    let cfg = PreflopConfig {
        bb_ante: 0.0,
        leaf_net: None,
        card_removal,
        limper_extra: 0.0,
        caller_extra: 0.0,
        raise_mults_oop: None,
        fourbet_mults_oop: None,
        utg_straddle: false,
        positions: vec!["SB".into(), "BB".into()],
        stack,
        stacks: None,
        posts: vec![0.5, 1.0],
        ante: 0.0,
        limp: false,
        open_raises: vec![],
        raise_mults: vec![],
        max_raises: 1,
        add_allin: true,
        allin_threshold: 0.85,
        rake_pct: 0.0,
        rake_cap: 0.0,
        no_flop_no_drop: true,
        realization: "raw".into(),
        call_only_seats: vec![],
        open_raises_by_seat: None,
        fourbet_mults: None,
        fourbet_mults_by_seat: None,
        raise_mults_by_seat: None,
        icm: None,
    };
    let eq = Arc::new(EquityTable::build(equity_samples));
    let mut s = PreflopSolver::new(cfg, eq).map_err(AdapterError::Backend)?;
    for _ in 0..iterations {
        s.iterate();
    }
    let find = |node: usize, kind: &str| {
        s.nodes[node]
            .actions
            .iter()
            .position(|a| a.kind == kind)
            .ok_or_else(|| AdapterError::Backend(format!("no {kind} action at preflop node {node}")))
    };
    let jam_a = find(0, "jam")?;
    let bb = s.child(0, jam_a);
    let call_a = find(bb, "call")?;
    let sb_strat = s.average_strategy(0);
    let bb_strat = s.average_strategy(bb);
    let mut out = ClassPushFold { jam: Default::default(), call: Default::default() };
    for h in 0..NUM_CLASSES {
        out.jam.insert(class_label(h), sb_strat[jam_a * NUM_CLASSES + h] as f64);
        out.call.insert(class_label(h), bb_strat[call_a * NUM_CLASSES + h] as f64);
    }
    Ok(out)
}
