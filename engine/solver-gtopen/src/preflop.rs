//! Preflop solving with GTOpen's preflop solver: an **approximate model**, not a full solve.
//!
//! GTOpen solves the preflop tree and values pots that reach the flop with a model, not by playing
//! the flop out. Heads-up flop pots get class-vs-class all-in equity times a realisation factor;
//! pots with three or more players use a showdown-rank approximation. Hands are grouped into
//! the 169 starting-hand classes, and card removal between players is approximated by GTOpen's
//! blocker model, which RiverLab always switches on.
//!
//! RiverLab has validated one case: heads-up push/fold, against its own exact combo-level
//! equilibrium (`docs/solver-validation.md`). Every other tree is labelled approximate.
//! [`PreflopOutcome::validated_scope`] says which applies.
//!
//! Chips: RiverLab's integer mbb become GTOpen's `f64` big blinds (`mbb / 1000`).

use crate::{AdapterError, BACKEND, ClassStrategy, GTOPEN_COMMIT, GTOPEN_REPOSITORY, StopReason};
use gtopen::preflop::equity::{EquityTable, NUM_CLASSES, class_combos, class_label};
use gtopen::preflop::{PreflopConfig, PreflopSolver, estimate_tree};
use poker_core::{Chips, GameConfig};
use range_engine::HandClass;
use serde::{Deserialize, Serialize};
use solver_interface::{EquityRealization, PreflopSpec};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

/// Monte Carlo samples per class pair in GTOpen's 169×169 equity table (its server default).
pub const EQUITY_SAMPLES: u32 = 20_000;

const POSITIONS: [&str; 6] = ["UTG", "HJ", "CO", "BTN", "SB", "BB"];

/// What RiverLab has independently validated about a preflop result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidatedScope {
    /// Heads-up push/fold without ante or rake: checked against RiverLab's exact equilibrium.
    HeadsUpPushFold,
    /// Anything else: an approximate model that RiverLab has not validated.
    ApproximateModel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreflopProgress {
    pub iterations: u64,
    /// Sum over players of what each gains by best-responding *inside the model*, in mbb/hand.
    pub model_gap_mbb: f64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreflopOutcome {
    pub backend: String,
    pub backend_repository: String,
    pub backend_commit: String,
    pub stop_reason: StopReason,
    pub progress: PreflopProgress,
    pub positions: Vec<String>,
    /// Per player, in mbb/hand, measured inside the model.
    pub gaps_mbb: Vec<f64>,
    /// Per player, in mbb/hand, under the model's payoffs.
    pub values_mbb: Vec<f64>,
    pub validated_scope: ValidatedScope,
    /// What the model leaves out or approximates.
    pub model_notes: Vec<String>,
    pub memory_bytes: u64,
    pub nodes: usize,
}

/// A GTOpen preflop configuration derived from a [`PreflopSpec`].
#[derive(Debug, Clone)]
pub struct PreparedPreflop {
    pub config: PreflopConfig,
    pub positions: Vec<String>,
    pub validated_scope: ValidatedScope,
    pub model_notes: Vec<String>,
}

fn bb(mbb: u64) -> f64 {
    mbb as f64 / 1000.0
}

/// Converts a validated [`PreflopSpec`] into a GTOpen preflop configuration.
pub fn prepare_preflop(spec: &PreflopSpec) -> Result<PreparedPreflop, AdapterError> {
    spec.validate().map_err(|e| AdapterError::InvalidSpec(e.to_string()))?;
    let game = &spec.game;
    let n = spec.players();
    let positions: Vec<String> = POSITIONS[6 - n..].iter().map(|s| s.to_string()).collect();
    let mut posts = vec![0.0; n];
    posts[n - 2] = bb(game.small_blind.mbb());
    posts[n - 1] = bb(GameConfig::BIG_BLIND.mbb());

    let mut notes = vec![
        "Approximate model: pots that reach the flop are valued by a model (equity × realisation \
         heads-up, a showdown-rank approximation with three or more players), not by solving the flop."
            .to_string(),
        "Hands are grouped into 169 classes; GTOpen's blocker model approximates card removal between players."
            .to_string(),
    ];

    // ---- rake ----
    let rake = &game.rake;
    let to_chips = |a: poker_core::rake::RakeAmount, what: &str| {
        a.to_chips(game.stake.as_ref(), what).map_err(|e| AdapterError::InvalidSpec(e.to_string()))
    };
    let cap = rake.cap.map(|c| to_chips(c, "rake cap")).transpose()?;
    let mut rake_pct = rake.percentage_bps as f64 / 100.0; // GTOpen preflop takes percent
    if cap == Some(Chips::ZERO) {
        rake_pct = 0.0;
    }
    if rake_pct > 0.0 {
        if rake.min_pot.is_some() {
            notes.push("The minimum raked pot is not modelled; every pot is raked.".into());
        }
        notes.push("Rake rounding is not modelled; GTOpen takes the exact percentage.".into());
        if !rake.no_flop_no_drop {
            notes.push("Pots won before the flop are raked (no-flop-no-drop is off).".into());
        }
    }
    notes.push(match spec.realization {
        EquityRealization::Full => "Every player realises exactly their all-in equity in flop pots.".into(),
        EquityRealization::Positional => "Flop pots use a fixed positional realisation adjustment \
            (in position realises more, out of position less, up to ±8% at deep stack-to-pot ratios)."
            .into(),
    });

    let s = &spec.sizing;
    let push_fold = n == 2
        && s.open_raise_to_mbb.is_empty()
        && s.add_all_in
        && s.max_raises == 1
        && !s.allow_limp
        && game.ante == Chips::ZERO
        && rake_pct == 0.0;
    let validated_scope = if push_fold { ValidatedScope::HeadsUpPushFold } else { ValidatedScope::ApproximateModel };
    if push_fold {
        // Every pot is all-in before the flop, so realisation never applies.
        notes = vec![
            "Heads-up push/fold: all-in pots need no flop model. RiverLab checked this case against its \
             exact combo-level equilibrium (10bb and 20bb: at most 0.00002bb/hand exploitable)."
                .into(),
        ];
    }

    let config = PreflopConfig {
        positions: positions.clone(),
        stack: bb(game.stack.effective_stack.mbb()),
        stacks: None,
        posts,
        utg_straddle: false,
        ante: bb(game.ante.mbb()),
        bb_ante: 0.0,
        limp: s.allow_limp,
        open_raises: s.open_raise_to_mbb.iter().map(|&o| bb(o)).collect(),
        raise_mults: s.reraise_pct_of_bet.iter().map(|&m| m as f64 / 100.0).collect(),
        max_raises: s.max_raises,
        add_allin: s.add_all_in,
        allin_threshold: s.all_in_threshold_pct_stack as f64 / 100.0,
        rake_pct,
        rake_cap: cap.map_or(0.0, |c| bb(c.mbb())),
        no_flop_no_drop: rake.no_flop_no_drop,
        realization: match spec.realization {
            EquityRealization::Full => "raw",
            EquityRealization::Positional => "static",
        }
        .into(),
        leaf_net: None,
        call_only_seats: vec![],
        open_raises_by_seat: None,
        raise_mults_by_seat: None,
        fourbet_mults: None,
        fourbet_mults_by_seat: None,
        icm: None,
        card_removal: true,
        limper_extra: 0.0,
        caller_extra: 0.0,
        raise_mults_oop: None,
        fourbet_mults_oop: None,
    };
    Ok(PreparedPreflop { config, positions, validated_scope, model_notes: notes })
}

/// GTOpen's class equity table, built once per process and cached on disk when `cache_dir` is set
/// (building takes about 30 s).
pub fn equity_table(cache_dir: Option<&Path>) -> Arc<EquityTable> {
    static TABLES: OnceLock<Mutex<HashMap<Option<PathBuf>, Arc<EquityTable>>>> = OnceLock::new();
    let key = cache_dir.map(Path::to_path_buf);
    let mut tables = TABLES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    tables
        .entry(key)
        .or_insert_with(|| {
            Arc::new(match cache_dir {
                Some(dir) => {
                    let path = dir.join(format!("gtopen_preflop_eq169_{EQUITY_SAMPLES}.bin"));
                    EquityTable::load_or_build(&path.to_string_lossy(), EQUITY_SAMPLES)
                }
                None => EquityTable::build(EQUITY_SAMPLES),
            })
        })
        .clone()
}

/// A finished preflop solve that can be browsed and saved.
pub struct PreflopSolved {
    solver: PreflopSolver,
    pub prepared: PreparedPreflop,
    pub outcome: PreflopOutcome,
}

fn gaps_values(s: &PreflopSolver) -> (Vec<f64>, Vec<f64>) {
    let (g, v) = s.gaps_and_evs();
    (g.iter().map(|x| x * 1000.0).collect(), v.iter().map(|x| x * 1000.0).collect())
}

/// Runs a preflop solve, refusing trees over `max_bytes` before allocating them. `cancel` is
/// checked inside iterations.
pub fn solve_preflop(
    spec: &PreflopSpec,
    eq: Arc<EquityTable>,
    max_bytes: u64,
    cancel: Arc<AtomicBool>,
    mut on_progress: impl FnMut(&PreflopProgress),
) -> Result<PreflopSolved, AdapterError> {
    let prepared = prepare_preflop(spec)?;
    let est = estimate_tree(&prepared.config).map_err(AdapterError::Backend)?;
    // Two f32 arenas plus per-node bookkeeping (GTOpen's own estimate counts 8 bytes per entry).
    let needed = est.arena_len * 8;
    if est.truncated || needed > max_bytes {
        return Err(AdapterError::TooLarge { needed_bytes: needed, limit_bytes: max_bytes });
    }
    let mut solver = PreflopSolver::new(prepared.config.clone(), eq).map_err(AdapterError::Backend)?;
    solver.set_stop_flag(Some(cancel.clone()));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(spec.threads as usize)
        .build()
        .map_err(|e| AdapterError::Backend(e.to_string()))?;
    let start = Instant::now();
    let mut progress = PreflopProgress { iterations: 0, model_gap_mbb: f64::NAN, elapsed_ms: 0 };
    let mut last = (Vec::new(), Vec::new());
    let reason = loop {
        if cancel.load(Ordering::Relaxed) || !pool.install(|| solver.try_iterate()) {
            break StopReason::Cancelled;
        }
        let it = solver.iteration as u64;
        let done = it >= spec.max_iterations;
        let timed_out = spec.max_seconds.is_some_and(|s| start.elapsed().as_secs() >= s);
        if done || timed_out || it % spec.check_interval == 0 {
            last = pool.install(|| gaps_values(&solver));
            if cancel.load(Ordering::Relaxed) {
                break StopReason::Cancelled; // values measured while stopping are discarded
            }
            progress = PreflopProgress {
                iterations: it,
                model_gap_mbb: last.0.iter().sum(),
                elapsed_ms: start.elapsed().as_millis() as u64,
            };
            on_progress(&progress);
        }
        if done {
            break StopReason::MaxIterations;
        }
        if timed_out {
            break StopReason::MaxSeconds;
        }
    };
    let (gaps_mbb, values_mbb) = last;
    let outcome = PreflopOutcome {
        backend: BACKEND.into(),
        backend_repository: GTOPEN_REPOSITORY.into(),
        backend_commit: GTOPEN_COMMIT.into(),
        stop_reason: reason,
        progress,
        positions: prepared.positions.clone(),
        gaps_mbb,
        values_mbb,
        validated_scope: prepared.validated_scope,
        model_notes: prepared.model_notes.clone(),
        memory_bytes: (solver.arena_mb() * 1e6) as u64,
        nodes: solver.nodes.len(),
    };
    Ok(PreflopSolved { solver, prepared, outcome })
}

/// Loads a preflop solve saved with [`PreflopSolved::save`] for browsing.
pub fn load_preflop(spec: &PreflopSpec, path: &Path, eq: Arc<EquityTable>) -> Result<PreflopBrowser, AdapterError> {
    let prepared = prepare_preflop(spec)?;
    let solver = PreflopSolver::load_game(&path.to_string_lossy(), eq).map_err(AdapterError::Backend)?;
    if solver.cfg.positions != prepared.config.positions || solver.cfg.stack != prepared.config.stack {
        return Err(AdapterError::Backend("saved preflop game does not match its specification".into()));
    }
    Ok(PreflopBrowser { solver })
}

/// Read-only access to a solved preflop tree.
pub struct PreflopBrowser {
    solver: PreflopSolver,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreflopAction {
    pub label: String,
    /// "fold", "call", "check", "raise", "jam", ...
    pub kind: String,
    /// Total the actor has put in after the action, in mbb.
    pub to_mbb: f64,
    /// Frequency over the actor's whole range at this node.
    pub frequency: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreflopStep {
    pub actor: String,
    pub actions: Vec<String>,
    pub chosen: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeatRange {
    pub position: String,
    /// Per class: the fraction of its combos still in the range (weight = combos × fraction).
    pub classes: Vec<ClassStrategy>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreflopNode {
    /// "action", "fold_win" (everyone else folded) or "pot_share" (the flop, or an all-in).
    pub kind: String,
    pub actor: Option<String>,
    pub positions: Vec<String>,
    pub pot_mbb: f64,
    pub invested_mbb: Vec<f64>,
    pub live: Vec<bool>,
    pub actions: Vec<PreflopAction>,
    /// Action nodes: the actor's strategy per class still in their range.
    pub by_class: Vec<ClassStrategy>,
    /// Ranges of the players still in the hand when they reach this node.
    pub ranges: Vec<SeatRange>,
    pub history: Vec<PreflopStep>,
    /// Set when GTOpen has no meaningful strategy here (e.g. the node is never reached).
    pub strategy_note: Option<String>,
    /// Probability of reaching this node under the model's independent-class deal.
    pub reach_probability: f64,
    pub stack_to_pot: Option<f64>,
}

fn class_grid(h: usize) -> Result<(HandClass, (usize, usize)), AdapterError> {
    let c = HandClass::from_str(&class_label(h)).map_err(|e| AdapterError::Backend(e.to_string()))?;
    Ok((c, c.grid_position()))
}

impl std::fmt::Debug for PreflopSolved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreflopSolved").field("outcome", &self.outcome).finish_non_exhaustive()
    }
}

impl std::fmt::Debug for PreflopBrowser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreflopBrowser").field("nodes", &self.solver.nodes.len()).finish_non_exhaustive()
    }
}

impl PreflopSolved {
    pub fn save(&self, path: &Path) -> Result<(), AdapterError> {
        self.solver.save_game(&path.to_string_lossy()).map_err(AdapterError::Backend)
    }

    pub fn node(&self, path: &[usize]) -> Result<PreflopNode, AdapterError> {
        node_view(&self.solver, path)
    }
}

impl PreflopBrowser {
    pub fn node(&self, path: &[usize]) -> Result<PreflopNode, AdapterError> {
        node_view(&self.solver, path)
    }
}

fn node_view(s: &PreflopSolver, path: &[usize]) -> Result<PreflopNode, AdapterError> {
    let v = s.node_view(path).map_err(AdapterError::InvalidSpec)?;
    let na = v.actions.len();
    let mut by_class = Vec::new();
    if let (Some(strategy), Some(reach)) = (&v.strategy, &v.reach) {
        for h in 0..NUM_CLASSES {
            let weight = (reach[h] * class_combos(h)) as f64;
            if weight <= 0.0 {
                continue;
            }
            let (class, (row, col)) = class_grid(h)?;
            by_class.push(ClassStrategy {
                label: class.to_string(),
                row,
                col,
                weight,
                frequencies: (0..na).map(|a| strategy[a * NUM_CLASSES + h] as f64).collect(),
            });
        }
    }
    let mut ranges = Vec::new();
    for (seat, reach) in v.reaches_all.iter().enumerate() {
        if reach.is_empty() || !v.live.get(seat).copied().unwrap_or(false) {
            continue;
        }
        let mut classes = Vec::new();
        for (h, &r) in reach.iter().enumerate().take(NUM_CLASSES) {
            if r <= 0.0 {
                continue;
            }
            let (class, (row, col)) = class_grid(h)?;
            classes.push(ClassStrategy {
                label: class.to_string(),
                row,
                col,
                weight: (r * class_combos(h)) as f64,
                frequencies: vec![r as f64],
            });
        }
        ranges.push(SeatRange { position: v.positions[seat].clone(), classes });
    }
    Ok(PreflopNode {
        kind: v.kind,
        actor: v.actor_pos,
        positions: v.positions,
        pot_mbb: v.pot * 1000.0,
        invested_mbb: v.invested.iter().map(|x| x * 1000.0).collect(),
        live: v.live,
        actions: v
            .actions
            .iter()
            .map(|a| PreflopAction {
                label: a.label.clone(),
                kind: a.kind.clone(),
                to_mbb: a.to * 1000.0,
                frequency: a.freq as f64,
            })
            .collect(),
        by_class,
        ranges,
        history: v
            .history
            .iter()
            .map(|h| PreflopStep {
                actor: h.actor_pos.clone(),
                actions: h.actions.iter().map(|a| a.label.clone()).collect(),
                chosen: h.chosen,
            })
            .collect(),
        strategy_note: v.strategy_note,
        reach_probability: v.branch_probability,
        stack_to_pot: v.spr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solver_interface::PreflopSizing;

    fn spec(heads_up: bool) -> PreflopSpec {
        PreflopSpec {
            game: GameConfig { players: 3, ..GameConfig::default() },
            heads_up,
            sizing: PreflopSizing {
                open_raise_to_mbb: vec![2_500],
                reraise_pct_of_bet: vec![300],
                max_raises: 2,
                add_all_in: true,
                all_in_threshold_pct_stack: 85,
                allow_limp: false,
            },
            realization: EquityRealization::Positional,
            max_iterations: 30,
            max_seconds: None,
            check_interval: 10,
            threads: 2,
        }
    }

    #[test]
    fn prepare_maps_blinds_rake_and_labels_scope() {
        let p = prepare_preflop(&spec(false)).unwrap();
        assert_eq!(p.positions, ["BTN", "SB", "BB"]);
        assert_eq!(p.config.posts, [0.0, 0.5, 1.0]);
        assert_eq!(p.config.rake_pct, 5.0);
        assert_eq!(p.config.open_raises, [2.5]);
        assert_eq!(p.config.raise_mults, [3.0]);
        assert!(p.config.card_removal);
        assert_eq!(p.validated_scope, ValidatedScope::ApproximateModel);
        assert!(p.model_notes[0].starts_with("Approximate model"));

        let mut pf = spec(true);
        pf.game.rake = poker_core::rake::RakeConfig::none();
        pf.sizing.open_raise_to_mbb.clear();
        pf.sizing.max_raises = 1;
        let p = prepare_preflop(&pf).unwrap();
        assert_eq!(p.positions, ["SB", "BB"]);
        assert_eq!(p.validated_scope, ValidatedScope::HeadsUpPushFold);
        pf.game.rake = poker_core::rake::RakeConfig::default();
        assert_eq!(prepare_preflop(&pf).unwrap().validated_scope, ValidatedScope::ApproximateModel);
    }

    #[test]
    fn solves_browses_saves_and_cancels() {
        let eq = Arc::new(EquityTable::build(200));
        let mut seen = 0;
        let s =
            solve_preflop(&spec(false), eq.clone(), 1 << 30, Arc::new(AtomicBool::new(false)), |_| seen += 1).unwrap();
        assert_eq!(seen, 3);
        assert_eq!(s.outcome.stop_reason, StopReason::MaxIterations);
        assert_eq!(s.outcome.gaps_mbb.len(), 3);
        assert!(s.outcome.progress.model_gap_mbb.is_finite());
        let root = s.node(&[]).unwrap();
        assert_eq!(root.kind, "action");
        assert_eq!(root.actor.as_deref(), Some("BTN"));
        assert_eq!(root.by_class.len(), 169);
        for c in &root.by_class {
            let total: f64 = c.frequencies.iter().sum();
            assert!((total - 1.0).abs() < 1e-3, "{} sums to {total}", c.label);
        }
        let fold = root.actions.iter().position(|a| a.kind == "fold").unwrap();
        let after = s.node(&[fold]).unwrap();
        assert_eq!(after.actor.as_deref(), Some("SB"));
        assert_eq!(after.history.len(), 2);
        assert_eq!(after.history[0].chosen, Some(fold));
        assert!(s.node(&[99]).is_err());

        let dir = std::env::temp_dir().join(format!("riverlab-preflop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("game.gtp");
        s.save(&file).unwrap();
        let loaded = load_preflop(&spec(false), &file, eq.clone()).unwrap();
        assert_eq!(loaded.node(&[fold]).unwrap(), after);
        assert!(load_preflop(&spec(true), &file, eq.clone()).is_err(), "spec mismatch is refused");
        std::fs::remove_dir_all(&dir).unwrap();

        let cancelled = solve_preflop(&spec(false), eq.clone(), 1 << 30, Arc::new(AtomicBool::new(true)), |_| {});
        assert_eq!(cancelled.unwrap().outcome.stop_reason, StopReason::Cancelled);
        let too_big = solve_preflop(&spec(false), eq, 1000, Arc::new(AtomicBool::new(false)), |_| {});
        assert!(matches!(too_big, Err(AdapterError::TooLarge { .. })));
    }
}
