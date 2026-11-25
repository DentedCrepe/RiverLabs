//! Python bindings: module `riverlab_engine`.
//!
//! Every function takes plain strings / numbers and returns a JSON string. Invalid input
//! raises `ValueError` with the engine's message; nothing is silently corrected.

use equity_engine::{DEFAULT_EXACT_BUDGET, EquityError};
use hand_evaluator::{best_five, evaluate};
use poker_core::game::{Action, GameState, Phase};
use poker_core::{Board, Card, CardSet, Chips, GameConfig, HoleCards, Position, StackPreset};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use range_engine::Range;
use serde::Deserialize;
use serde_json::{Value, json};
use solver_interface::{SolveSpec, SolverConfig};

fn err<E: std::fmt::Display>(e: E) -> PyErr {
    PyValueError::new_err(e.to_string())
}

fn to_json(v: &impl serde::Serialize) -> PyResult<String> {
    serde_json::to_string(v).map_err(err)
}

fn parse_cards(s: &str) -> PyResult<Vec<Card>> {
    Card::parse_many(s).map_err(err)
}

fn parse_dead(s: &str) -> PyResult<CardSet> {
    let cards = parse_cards(s)?;
    CardSet::from_distinct(&cards).map_err(|c| err(format!("duplicate card {c}")))
}

/// Engine crate version, recorded with every persisted result.
#[pyfunction]
fn engine_version() -> &'static str {
    poker_core::ENGINE_VERSION
}

/// Evaluates 5–7 cards, e.g. "AsKsQsJsTs2c3d".
#[pyfunction]
fn evaluate_hand(cards: &str) -> PyResult<String> {
    let cards = parse_cards(cards)?;
    let s = evaluate(&cards).map_err(err)?;
    let (five, _) = best_five(&cards).map_err(err)?;
    to_json(&json!({
        "value": s.value(),
        "category": s.category(),
        "category_name": s.category().name(),
        "description": s.describe(),
        "best_five": five.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
    }))
}

/// Showdown between hole-card holdings on a complete five-card board. All cards across
/// the board and every hand must be distinct.
#[pyfunction]
fn compare_hands(board: &str, hands: Vec<String>) -> PyResult<String> {
    let board: Board = board.parse().map_err(err)?;
    if board.cards().len() != 5 {
        return Err(err("the board must contain exactly five cards"));
    }
    if hands.len() < 2 {
        return Err(err("at least two hands are required"));
    }
    let mut used = board.card_set();
    let mut results = Vec::with_capacity(hands.len());
    for h in &hands {
        let hole: HoleCards = h.parse().map_err(err)?;
        for c in hole.cards() {
            if !used.insert(c) {
                return Err(err(format!("card {c} appears more than once")));
            }
        }
        let mut cards = board.cards().to_vec();
        cards.extend(hole.cards());
        let s = evaluate(&cards).map_err(err)?;
        let (five, _) = best_five(&cards).map_err(err)?;
        results.push((s, five));
    }
    let best = results.iter().map(|(s, _)| *s).max().expect("non-empty");
    let winners: Vec<usize> = results.iter().enumerate().filter(|(_, (s, _))| *s == best).map(|(i, _)| i).collect();
    let results: Vec<Value> = results
        .into_iter()
        .map(|(s, five)| {
            json!({
                "value": s.value(),
                "category": s.category(),
                "category_name": s.category().name(),
                "description": s.describe(),
                "best_five": five.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
            })
        })
        .collect();
    to_json(&json!({ "results": results, "winners": winners }))
}

/// Parses range notation and returns canonical notation, counts and the 13×13 matrix.
#[pyfunction]
#[pyo3(signature = (notation, dead = ""))]
fn analyze_range(notation: &str, dead: &str) -> PyResult<String> {
    let r = Range::parse(notation).map_err(err)?;
    let dead = parse_dead(dead)?;
    to_json(&json!({
        "notation": r.to_notation(),
        "combos": r.combo_count(),
        "live_combos": r.live_combo_count(dead),
        "fraction_of_all_hands": r.fraction_of_all_hands(),
        "blocked_fraction": r.blocked_fraction(dead),
        "matrix": r.matrix(dead),
    }))
}

/// Equity for 2–6 ranges (hands are single-combo ranges such as "AsKh").
#[pyfunction]
#[pyo3(signature = (ranges, board = "", dead = "", method = "exact", iterations = 100_000, seed = 0, exact_budget = DEFAULT_EXACT_BUDGET))]
#[allow(clippy::too_many_arguments)]
fn equity(
    py: Python<'_>,
    ranges: Vec<String>,
    board: &str,
    dead: &str,
    method: &str,
    iterations: u64,
    seed: u64,
    exact_budget: u64,
) -> PyResult<String> {
    let ranges: Vec<Range> = ranges.iter().map(|r| Range::parse(r).map_err(err)).collect::<PyResult<_>>()?;
    let board: Board = board.parse().map_err(err)?;
    let dead = parse_dead(dead)?;
    if dead.intersects(board.card_set()) {
        return Err(err("dead cards overlap the board"));
    }
    if method == "monte_carlo" && iterations > 50_000_000 {
        return Err(err("iterations must be at most 50,000,000"));
    }
    // Release the GIL: enumeration can take seconds and must not block other Python threads.
    let result: Result<_, EquityError> = match method {
        "exact" => py.detach(|| equity_engine::exact(&ranges, &board, dead, exact_budget)),
        "monte_carlo" => py.detach(|| equity_engine::monte_carlo(&ranges, &board, dead, iterations, seed)),
        other => return Err(err(format!("unknown method '{other}' (expected exact or monte_carlo)"))),
    };
    to_json(&result.map_err(err)?)
}

/// Validates a game configuration JSON document and returns it in canonical form.
#[pyfunction]
fn validate_game_config(config_json: &str) -> PyResult<String> {
    let c: GameConfig = serde_json::from_str(config_json).map_err(err)?;
    c.validate().map_err(err)?;
    to_json(&c)
}

#[pyfunction]
fn default_game_config() -> PyResult<String> {
    to_json(&GameConfig::default())
}

/// Rake for a final pot (mbb, after uncalled bets are returned) under a game config.
#[pyfunction]
fn compute_rake(config_json: &str, pot_mbb: u64, saw_flop: bool) -> PyResult<String> {
    let c: GameConfig = serde_json::from_str(config_json).map_err(err)?;
    c.validate().map_err(err)?;
    to_json(&c.rake.compute(Chips(pot_mbb), saw_flop, c.stake.as_ref()).map_err(err)?)
}

#[pyfunction]
fn stack_presets() -> PyResult<String> {
    let v: Vec<Value> = StackPreset::ALL
        .iter()
        .map(|p| json!({"label": p.label(), "big_blinds": p.big_blinds(), "mbb": p.chips().mbb()}))
        .collect();
    to_json(&json!({
        "presets": v,
        "default": StackPreset::Bb100.label(),
        "min_mbb": poker_core::stack::MIN_EFFECTIVE_STACK.mbb(),
        "max_mbb": poker_core::stack::MAX_EFFECTIVE_STACK.mbb(),
    }))
}

#[pyfunction]
#[pyo3(signature = (players = 6))]
fn positions(players: usize) -> PyResult<String> {
    let ps = Position::for_players(players).map_err(err)?;
    let post = Position::postflop_order(players);
    let v: Vec<Value> = ps
        .iter()
        .enumerate()
        .map(|(i, p)| {
            json!({
                "code": p.short_name(),
                "name": p.long_name(),
                "preflop_order": i,
                "postflop_order": post.iter().position(|&s| s == i),
                "is_blind": p.is_blind(),
            })
        })
        .collect();
    to_json(&v)
}

#[pyfunction]
fn validate_solver_config(config_json: &str) -> PyResult<String> {
    let c: SolverConfig = serde_json::from_str(config_json).map_err(err)?;
    c.validate().map_err(err)?;
    to_json(&c)
}

#[pyfunction]
fn validate_solve_spec(spec_json: &str) -> PyResult<String> {
    let s: SolveSpec = serde_json::from_str(spec_json).map_err(err)?;
    s.validate().map_err(err)?;
    for r in &s.ranges {
        Range::parse(r).map_err(err)?;
    }
    to_json(&s)
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum HandEvent {
    HoleCards { seat: usize, cards: HoleCards },
    Action { action: Action },
    Board { cards: String },
}

/// Replays a hand from a config and a list of events, returning the resulting state, legal
/// actions and — when the hand is over and all live hole cards are known — the settlement.
#[pyfunction]
#[pyo3(signature = (config_json, events_json, stacks_mbb = None))]
fn replay_hand(config_json: &str, events_json: &str, stacks_mbb: Option<Vec<u64>>) -> PyResult<String> {
    let config: GameConfig = serde_json::from_str(config_json).map_err(err)?;
    let events: Vec<HandEvent> = serde_json::from_str(events_json).map_err(err)?;
    let mut g = match stacks_mbb {
        Some(s) => GameState::with_stacks(config, &s.into_iter().map(Chips).collect::<Vec<_>>()),
        None => GameState::new(config),
    }
    .map_err(err)?;
    for (i, e) in events.into_iter().enumerate() {
        let r = match e {
            HandEvent::HoleCards { seat, cards } => g.set_hole_cards(seat, cards),
            HandEvent::Action { action } => g.apply(action),
            HandEvent::Board { cards } => g.deal_board(&parse_cards(&cards)?),
        };
        r.map_err(|e| err(format!("event {i}: {e}")))?;
    }
    let settlement = match g.phase() {
        Phase::Uncontested { .. } => Some(g.settle_uncontested().map_err(err)?),
        Phase::Showdown => {
            let board = g.board().cards().to_vec();
            let values: Option<Vec<Option<u32>>> = g
                .players()
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    if p.folded {
                        Some(None)
                    } else {
                        g.hole_cards(i).map(|h| {
                            let mut cards = board.clone();
                            cards.extend(h.cards());
                            Some(evaluate(&cards).expect("validated distinct cards").value())
                        })
                    }
                })
                .collect();
            match values {
                Some(v) => Some(g.settle_showdown(&v).map_err(err)?),
                None => None,
            }
        }
        _ => None,
    };
    to_json(&json!({
        "phase": g.phase(),
        "street": g.street(),
        "board": g.board(),
        "pot_mbb": g.pot_total(),
        "current_bet_mbb": g.current_bet(),
        "min_raise_increment_mbb": g.min_raise_increment(),
        "players": g.players(),
        "pots": g.pots(),
        "legal_actions": g.legal_actions().ok(),
        "history": g.history(),
        "settlement": settlement,
    }))
}

/// Cooperative cancellation for a running solve. Thread-safe.
#[pyclass(frozen)]
struct CancelToken {
    flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[pymethods]
impl CancelToken {
    #[new]
    fn new() -> Self {
        CancelToken { flag: Default::default() }
    }
    fn cancel(&self) {
        self.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    fn is_cancelled(&self) -> bool {
        self.flag.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// A solved postflop game loaded from storage, for browsing its tree.
#[pyclass(frozen)]
struct SolvedGame {
    inner: std::sync::Mutex<solver_gtopen::Solved>,
}

#[pymethods]
impl SolvedGame {
    /// The node at `path_json` (a JSON list of {"type": "action", "value": i} /
    /// {"type": "card", "value": "Ah"} steps), as JSON.
    fn node(&self, py: Python<'_>, path_json: &str) -> PyResult<String> {
        let path: Vec<solver_gtopen::PathStep> = serde_json::from_str(path_json).map_err(err)?;
        let view = py.detach(|| {
            let solved = self.inner.lock().expect("solved game lock");
            solved.node_view(&path)
        });
        to_json(&view.map_err(err)?)
    }

    /// Deals a postflop drill from this solve (`hero`: 0 out of position, 1 in position, or None).
    #[pyo3(signature = (spec_json, seed, hero=None))]
    fn drill(&self, py: Python<'_>, spec_json: &str, seed: u64, hero: Option<u8>) -> PyResult<String> {
        let spec: SolveSpec = serde_json::from_str(spec_json).map_err(err)?;
        let d = py.detach(|| {
            let solved = self.inner.lock().expect("solved game lock");
            play_engine::drill::deal(&solved, &spec, seed, hero)
        });
        to_json(&d.map_err(play_err)?)
    }

    /// Grades action `choice` for a drill dealt from this solve.
    fn grade(&self, py: Python<'_>, spec_json: &str, drill_json: &str, choice: usize) -> PyResult<String> {
        let spec: SolveSpec = serde_json::from_str(spec_json).map_err(err)?;
        let drill: play_engine::drill::PostflopDrill = serde_json::from_str(drill_json).map_err(err)?;
        let g = py.detach(|| {
            let solved = self.inner.lock().expect("solved game lock");
            play_engine::drill::grade(&solved, &spec, &drill, choice)
        });
        to_json(&g.map_err(play_err)?)
    }
}

/// Loads a solve saved with `solve_postflop(..., save_path=...)` (GIL released).
#[pyfunction]
fn load_solved(py: Python<'_>, spec_json: &str, path: &str) -> PyResult<SolvedGame> {
    let spec: SolveSpec = serde_json::from_str(spec_json).map_err(err)?;
    let solved = py.detach(|| solver_gtopen::load(&spec, path)).map_err(|e| match e {
        solver_gtopen::AdapterError::Backend(m) => pyo3::exceptions::PyRuntimeError::new_err(m),
        other => err(other),
    })?;
    Ok(SolvedGame { inner: std::sync::Mutex::new(solved) })
}

/// Description of the available solver backend(s).
#[pyfunction]
fn solver_backends() -> PyResult<String> {
    to_json(&json!([{
        "name": solver_gtopen::BACKEND,
        "repository": solver_gtopen::GTOPEN_REPOSITORY,
        "commit": solver_gtopen::GTOPEN_COMMIT,
        "patches": solver_gtopen::GTOPEN_PATCHES,
        "scope": "heads-up postflop (flop, turn or river root); preflop as an approximate model",
    }]))
}

/// Solves a postflop `SolveSpec` (JSON) with GTOpen. Releases the GIL while solving.
/// `progress(json)` is called at each exploitability check. Raises `ValueError` for specs the
/// backend rejects (invalid, unsupported or too large) and `RuntimeError` for solver failures.
#[pyfunction]
#[pyo3(signature = (spec_json, cancel, progress = None, max_bytes = 2u64 << 30, threads = None, save_path = None))]
fn solve_postflop(
    py: Python<'_>,
    spec_json: &str,
    cancel: &CancelToken,
    progress: Option<Py<PyAny>>,
    max_bytes: u64,
    threads: Option<u16>,
    save_path: Option<String>,
) -> PyResult<String> {
    use solver_gtopen::AdapterError;
    let mut spec: SolveSpec = serde_json::from_str(spec_json).map_err(err)?;
    if let Some(t) = threads {
        spec.solver.threads = t;
    }
    let flag = cancel.flag.clone();
    let limits = solver_gtopen::Limits { max_bytes, ..Default::default() };
    let result = py.detach(|| {
        solver_gtopen::solve_with_limits(&spec, limits, &flag, |p| {
            if let Some(cb) = &progress {
                let payload = serde_json::to_string(p).expect("progress serializes");
                Python::attach(|py| {
                    if let Err(e) = cb.call1(py, (payload,)) {
                        // Never swallow: surface the callback's error and keep solving.
                        e.print(py);
                    }
                });
            }
        })
    });
    match result {
        Ok(mut solved) => {
            if let Some(path) = save_path {
                if solved.outcome.stop_reason != solver_gtopen::StopReason::Cancelled {
                    py.detach(|| solved.save(&path)).map_err(|e| {
                        pyo3::exceptions::PyRuntimeError::new_err(format!("saving the solve failed: {e}"))
                    })?;
                }
            }
            to_json(&solved.outcome)
        }
        Err(e @ (AdapterError::InvalidSpec(_) | AdapterError::Unsupported(_) | AdapterError::TooLarge { .. })) => {
            Err(err(e))
        }
        Err(e @ AdapterError::Backend(_)) => Err(pyo3::exceptions::PyRuntimeError::new_err(e.to_string())),
    }
}

/// Validates a preflop `PreflopSpec` (JSON) and returns it in canonical form.
#[pyfunction]
fn validate_preflop_spec(spec_json: &str) -> PyResult<String> {
    let spec: solver_interface::PreflopSpec = serde_json::from_str(spec_json).map_err(err)?;
    solver_gtopen::preflop::prepare_preflop(&spec).map_err(err)?;
    to_json(&spec)
}

/// A solved preflop game (approximate model), for browsing its tree.
#[pyclass(frozen)]
struct PreflopGame {
    inner: solver_gtopen::preflop::PreflopBrowser,
}

#[pymethods]
impl PreflopGame {
    /// The node reached by `path` (action indices from the root), as JSON.
    fn node(&self, py: Python<'_>, path: Vec<usize>) -> PyResult<String> {
        let view = py.detach(|| self.inner.node(&path));
        to_json(&view.map_err(err)?)
    }

    /// Deals a preflop drill (`spot`: "open", "facing_raise" or "any") from this solve, as JSON.
    fn drill(&self, py: Python<'_>, spot: &str, seed: u64) -> PyResult<String> {
        let spot: play_engine::preflop_drill::PreflopSpot =
            serde_json::from_value(serde_json::Value::String(spot.into())).map_err(err)?;
        let d = py.detach(|| play_engine::preflop_drill::deal(&self.inner, spot, seed));
        to_json(&d.map_err(play_err)?)
    }

    /// Grades an action index for a drill dealt by `drill`, against the model's strategy.
    fn grade(&self, py: Python<'_>, drill_json: &str, choice: usize) -> PyResult<String> {
        let drill: play_engine::preflop_drill::PreflopDrill = serde_json::from_str(drill_json).map_err(err)?;
        let g = py.detach(|| play_engine::preflop_drill::grade(&self.inner, &drill, choice));
        to_json(&g.map_err(play_err)?)
    }
}

fn preflop_error(e: solver_gtopen::AdapterError) -> PyErr {
    match e {
        solver_gtopen::AdapterError::Backend(m) => pyo3::exceptions::PyRuntimeError::new_err(m),
        other => err(other),
    }
}

/// Solves a `PreflopSpec` (JSON) with GTOpen's preflop model. Releases the GIL while solving.
/// `equity_cache_dir` keeps GTOpen's class equity table between runs (it takes ~30 s to build).
/// Raises `ValueError` for rejected specs and `RuntimeError` for solver failures.
#[pyfunction]
#[pyo3(signature = (spec_json, cancel, progress = None, max_bytes = 2u64 << 30, threads = None, save_path = None, equity_cache_dir = None))]
#[allow(clippy::too_many_arguments)]
fn solve_preflop(
    py: Python<'_>,
    spec_json: &str,
    cancel: &CancelToken,
    progress: Option<Py<PyAny>>,
    max_bytes: u64,
    threads: Option<u16>,
    save_path: Option<String>,
    equity_cache_dir: Option<String>,
) -> PyResult<String> {
    let mut spec: solver_interface::PreflopSpec = serde_json::from_str(spec_json).map_err(err)?;
    if let Some(t) = threads {
        spec.threads = t;
    }
    let flag = cancel.flag.clone();
    let result = py.detach(|| {
        let eq = solver_gtopen::preflop::equity_table(equity_cache_dir.as_deref().map(std::path::Path::new));
        solver_gtopen::preflop::solve_preflop(&spec, eq, max_bytes, flag, |p| {
            if let Some(cb) = &progress {
                let payload = serde_json::to_string(p).expect("progress serializes");
                Python::attach(|py| {
                    if let Err(e) = cb.call1(py, (payload,)) {
                        e.print(py);
                    }
                });
            }
        })
    });
    let solved = result.map_err(preflop_error)?;
    if let Some(path) = save_path {
        if solved.outcome.stop_reason != solver_gtopen::StopReason::Cancelled {
            py.detach(|| solved.save(std::path::Path::new(&path)))
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("saving the solve failed: {e}")))?;
        }
    }
    to_json(&json!({ "outcome": solved.outcome, "root": solved.node(&[]).map_err(preflop_error)? }))
}

/// Loads a preflop solve saved with `solve_preflop(..., save_path=...)` (GIL released).
#[pyfunction]
#[pyo3(signature = (spec_json, path, equity_cache_dir = None))]
fn load_preflop(
    py: Python<'_>,
    spec_json: &str,
    path: &str,
    equity_cache_dir: Option<String>,
) -> PyResult<PreflopGame> {
    let spec: solver_interface::PreflopSpec = serde_json::from_str(spec_json).map_err(err)?;
    let inner = py
        .detach(|| {
            let eq = solver_gtopen::preflop::equity_table(equity_cache_dir.as_deref().map(std::path::Path::new));
            solver_gtopen::preflop::load_preflop(&spec, std::path::Path::new(path), eq)
        })
        .map_err(preflop_error)?;
    Ok(PreflopGame { inner })
}

fn play_err(e: play_engine::PlayError) -> PyErr {
    match e {
        play_engine::PlayError::Engine(m) => pyo3::exceptions::PyRuntimeError::new_err(m),
        other => pyo3::exceptions::PyValueError::new_err(other.to_string()),
    }
}

fn model_cache(dir: Option<String>, max_bytes: u64, threads: u16) -> std::sync::Arc<play_engine::ModelCache> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    static CACHES: OnceLock<Mutex<HashMap<Option<String>, Arc<play_engine::ModelCache>>>> = OnceLock::new();
    let mut caches = CACHES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    caches
        .entry(dir.clone())
        .or_insert_with(|| Arc::new(play_engine::ModelCache::new(dir.map(Into::into), max_bytes, threads)))
        .clone()
}

/// Validates a play `TableConfig` (JSON) and returns it in canonical form.
#[pyfunction]
fn validate_table_config(config_json: &str) -> PyResult<String> {
    let cfg: play_engine::TableConfig = serde_json::from_str(config_json).map_err(err)?;
    cfg.validate().map_err(play_err)?;
    to_json(&cfg)
}

/// A game against bots. Methods release the GIL; the first hand at a new table shape can take
/// tens of seconds while the bots' preflop model is solved (call `prepare` first).
#[pyclass(frozen)]
struct PlaySession {
    inner: std::sync::Mutex<play_engine::Session>,
    models: std::sync::Arc<play_engine::ModelCache>,
}

#[pymethods]
impl PlaySession {
    #[new]
    #[pyo3(signature = (config_json, model_cache_dir = None, max_bytes = 1u64 << 30, threads = 2))]
    fn new(config_json: &str, model_cache_dir: Option<String>, max_bytes: u64, threads: u16) -> PyResult<Self> {
        let cfg: play_engine::TableConfig = serde_json::from_str(config_json).map_err(err)?;
        let session = play_engine::Session::new(cfg).map_err(play_err)?;
        Ok(PlaySession {
            inner: std::sync::Mutex::new(session),
            models: model_cache(model_cache_dir, max_bytes, threads),
        })
    }

    fn prepare(&self, py: Python<'_>) -> PyResult<()> {
        py.detach(|| self.lock().prepare(&self.models)).map_err(play_err)
    }

    /// While you are to act: your equity and what it is made of (JSON `EquityBreakdown`).
    fn equity_breakdown(&self, py: Python<'_>) -> PyResult<String> {
        py.detach(|| self.lock().equity_breakdown()).map_err(play_err).and_then(|b| to_json(&b))
    }

    /// Deals the next hand; returns the table view (JSON).
    fn next_hand(&self, py: Python<'_>) -> PyResult<String> {
        py.detach(|| {
            let mut s = self.lock();
            s.start_hand(&self.models)?;
            Ok(s.view())
        })
        .map_err(play_err)
        .and_then(|v| to_json(&v))
    }

    /// Your action (JSON, e.g. {"type": "raise", "to": 2500}); returns the table view.
    fn act(&self, py: Python<'_>, action_json: &str) -> PyResult<String> {
        let action: poker_core::game::Action = serde_json::from_str(action_json).map_err(err)?;
        py.detach(|| {
            let mut s = self.lock();
            s.act(action)?;
            Ok(s.view())
        })
        .map_err(play_err)
        .and_then(|v| to_json(&v))
    }

    fn view(&self) -> PyResult<String> {
        to_json(&self.lock().view())
    }

    /// Practice mode: show the bots' cards during hands (flagged in the report). Returns the view.
    fn set_reveal(&self, on: bool) -> PyResult<String> {
        let mut s = self.lock();
        s.set_reveal(on);
        to_json(&s.view())
    }

    fn leave(&self) -> PyResult<String> {
        let mut s = self.lock();
        s.leave();
        to_json(&s.view())
    }

    /// Hand records from `start` (0-based) on, as a JSON list.
    #[pyo3(signature = (start = 0))]
    fn records(&self, start: usize) -> PyResult<String> {
        let s = self.lock();
        to_json(&s.records().get(start..).unwrap_or(&[]))
    }

    fn hands_played(&self) -> u64 {
        self.lock().hands_played()
    }

    fn status(&self) -> PyResult<String> {
        to_json(&self.lock().status())
    }
}

impl PlaySession {
    fn lock(&self) -> std::sync::MutexGuard<'_, play_engine::Session> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Builds a session report from stored hand records (JSON list).
#[pyfunction]
fn play_report(py: Python<'_>, records_json: &str, tournament: bool) -> PyResult<String> {
    let records: Vec<play_engine::HandRecord> = serde_json::from_str(records_json).map_err(err)?;
    let report = py.detach(|| play_engine::report::build(&records, tournament));
    to_json(&report)
}

/// "Solve this spot": the postflop solve for decision `index` of a hand record (JSON), using the
/// session's table config (JSON) for the rake, and the richest tree that fits in `max_bytes`.
/// Returns `play_engine::spot::SpotExport` as JSON.
#[pyfunction]
fn play_spot(py: Python<'_>, record_json: &str, index: usize, config_json: &str, max_bytes: u64) -> PyResult<String> {
    let record: play_engine::HandRecord = serde_json::from_str(record_json).map_err(err)?;
    let cfg: play_engine::TableConfig = serde_json::from_str(config_json).map_err(err)?;
    let export = py.detach(|| play_engine::spot::export(&record, index, &cfg, max_bytes)).map_err(play_err)?;
    to_json(&export)
}

/// Builds the exact heads-up all-in equity table for the push/fold trainer and saves it to `path`
/// (tens of minutes; about 47k exact enumerations). `progress(done, total)` is called as it goes.
#[pyfunction]
fn push_fold_build_table(py: Python<'_>, path: &str, progress: Py<PyAny>) -> PyResult<()> {
    let path = std::path::PathBuf::from(path);
    py.detach(|| {
        let t = solver_validation::push_fold::ComboEquities::compute_with_progress(|done, total| {
            Python::attach(|py| {
                if let Err(e) = progress.call1(py, (done, total)) {
                    e.print(py);
                }
            });
        });
        t.save(&path)
    })
    .map_err(err)
}

/// Push/fold drills graded against the exact heads-up equilibrium. Loading verifies the table
/// (structure, symmetry and `samples` entries re-computed exactly).
#[pyclass(frozen)]
struct PushFoldTrainer {
    inner: play_engine::push_fold::PushFoldTrainer,
}

#[pymethods]
impl PushFoldTrainer {
    #[new]
    fn new(py: Python<'_>, path: &str, samples: usize) -> PyResult<Self> {
        let path = std::path::PathBuf::from(path);
        let eq = py.detach(|| {
            let eq = solver_validation::push_fold::ComboEquities::load(&path)
                .ok_or_else(|| format!("{} is missing or is not an equity table", path.display()))?;
            eq.verify(samples, 0x5EED).map(|()| eq)
        });
        let eq = eq.map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("equity table rejected: {e}")))?;
        Ok(PushFoldTrainer { inner: play_engine::push_fold::PushFoldTrainer::new(std::sync::Arc::new(eq)) })
    }

    /// `seat`: "sb", "bb" or None (random). Stacks in mbb.
    #[pyo3(signature = (min_mbb, max_mbb, seat, seed))]
    fn drill(&self, py: Python<'_>, min_mbb: u64, max_mbb: u64, seat: Option<&str>, seed: u64) -> PyResult<String> {
        let seat =
            seat.map(|s| serde_json::from_value(serde_json::Value::String(s.into()))).transpose().map_err(err)?;
        to_json(&py.detach(|| self.inner.drill(min_mbb, max_mbb, seat, seed)).map_err(play_err)?)
    }

    /// `drill_json`: a drill; `choice`: "fold" or "play" (jam or call). Solves the depth on first use.
    fn grade(&self, py: Python<'_>, drill_json: &str, choice: &str) -> PyResult<String> {
        let drill: play_engine::push_fold::Drill = serde_json::from_str(drill_json).map_err(err)?;
        let choice = serde_json::from_value(serde_json::Value::String(choice.into())).map_err(err)?;
        to_json(&py.detach(|| self.inner.grade(&drill, choice)).map_err(play_err)?)
    }

    fn chart(&self, py: Python<'_>, stack_mbb: u64) -> PyResult<String> {
        to_json(&py.detach(|| self.inner.chart(stack_mbb)).map_err(play_err)?)
    }
}

/// Solver review of a whole hand: the streets you made decisions on, and which can be solved.
#[pyfunction]
fn review_streets(record_json: &str) -> PyResult<String> {
    let record: play_engine::HandRecord = serde_json::from_str(record_json).map_err(err)?;
    to_json(&play_engine::review::streets(&record))
}

/// The solve for one street of a hand review. `carried_json`: `[oop, ip]` ranges from the end of
/// the previous solved street, or None to start from the preflop model and the tracked range.
#[pyfunction]
#[pyo3(signature = (record_json, street, config_json, max_bytes, carried_json=None))]
fn review_street_spec(
    py: Python<'_>,
    record_json: &str,
    street: &str,
    config_json: &str,
    max_bytes: u64,
    carried_json: Option<&str>,
) -> PyResult<String> {
    let record: play_engine::HandRecord = serde_json::from_str(record_json).map_err(err)?;
    let cfg: play_engine::TableConfig = serde_json::from_str(config_json).map_err(err)?;
    let carried: Option<[String; 2]> = carried_json.map(serde_json::from_str).transpose().map_err(err)?;
    let spec =
        py.detach(|| play_engine::review::street_spec(&record, street, &cfg, max_bytes, carried)).map_err(play_err)?;
    to_json(&spec)
}

/// Follows a street's real line through its saved solve (GIL released).
#[pyfunction]
fn review_analyze(py: Python<'_>, street_spec_json: &str, path: &str) -> PyResult<String> {
    let s: play_engine::review::StreetSpec = serde_json::from_str(street_spec_json).map_err(err)?;
    let review = py.detach(|| {
        let solved = solver_gtopen::load(&s.spec, path).map_err(|e| play_engine::PlayError::Engine(e.to_string()))?;
        play_engine::review::analyze(&solved, &s)
    });
    to_json(&review.map_err(play_err)?)
}

/// Academy theory calculations. `kind`: pot_odds, bet_math, ev_call, ev_bluff, outs or
/// regret_matching; `params_json` holds that calculation's inputs. Returns JSON.
#[pyfunction]
fn theory(kind: &str, params_json: &str) -> PyResult<String> {
    use poker_theory as t;
    let p: serde_json::Value = serde_json::from_str(params_json).map_err(err)?;
    let u = |k: &str| -> PyResult<u64> {
        p.get(k).and_then(serde_json::Value::as_u64).ok_or_else(|| err(format!("missing whole-number field '{k}'")))
    };
    let f = |k: &str| -> PyResult<f64> {
        p.get(k).and_then(serde_json::Value::as_f64).ok_or_else(|| err(format!("missing number field '{k}'")))
    };
    let small = |k: &str| -> PyResult<u32> { u32::try_from(u(k)?).map_err(|_| err(format!("'{k}' is too large"))) };
    let out = match kind {
        "pot_odds" => serde_json::to_value(t::pot_odds(u("pot_mbb")?, u("to_call_mbb")?).map_err(err)?),
        "bet_math" => serde_json::to_value(t::bet_math(u("pot_mbb")?, u("bet_mbb")?).map_err(err)?),
        "ev_call" => serde_json::to_value(t::ev_call(f("equity")?, u("pot_mbb")?, u("to_call_mbb")?).map_err(err)?),
        "ev_bluff" => {
            serde_json::to_value(t::ev_bluff(f("fold_frequency")?, u("pot_mbb")?, u("bet_mbb")?).map_err(err)?)
        }
        "outs" => serde_json::to_value(t::outs(small("outs")?, small("unseen")?).map_err(err)?),
        "implied_odds" => {
            serde_json::to_value(t::implied_odds(u("pot_mbb")?, u("to_call_mbb")?, f("equity")?).map_err(err)?)
        }
        "semi_bluff" => serde_json::to_value(
            t::semi_bluff(u("pot_mbb")?, u("bet_mbb")?, f("fold_frequency")?, f("equity")?).map_err(err)?,
        ),
        "zero_sum_2x2" => {
            let m: [[f64; 2]; 2] = p
                .get("payoff")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .ok_or_else(|| err("missing 'payoff' (a 2×2 list of numbers)"))?;
            serde_json::to_value(t::zero_sum_2x2(m).map_err(err)?)
        }
        "regret_matching" => {
            let start: Vec<f64> = p
                .get("start_regrets")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .ok_or_else(|| err("missing 'start_regrets' (three numbers)"))?;
            let start: [f64; 3] = start.try_into().map_err(|_| err("'start_regrets' must have three numbers"))?;
            let plus = p.get("plus").and_then(serde_json::Value::as_bool).unwrap_or(false);
            serde_json::to_value(t::regret_matching_rps(small("iterations")?, start, plus).map_err(err)?)
        }
        "icm" | "icm_call" => {
            let list = |k: &str| p.get(k).and_then(|v| v.as_array()).ok_or_else(|| err(format!("missing list '{k}'")));
            let stacks: Vec<u64> = list("stacks")?
                .iter()
                .map(|v| v.as_u64().ok_or_else(|| err("stacks must be whole numbers of chips")))
                .collect::<PyResult<_>>()?;
            let prizes: Vec<f64> = list("prizes")?
                .iter()
                .map(|v| v.as_f64().ok_or_else(|| err("prizes must be numbers")))
                .collect::<PyResult<_>>()?;
            if kind == "icm" {
                serde_json::to_value(t::icm::icm(&stacks, &prizes).map_err(err)?)
            } else {
                let i = |k: &str| -> PyResult<usize> {
                    usize::try_from(u(k)?).map_err(|_| err(format!("'{k}' is too large")))
                };
                serde_json::to_value(
                    t::icm::icm_call(&stacks, i("hero")?, i("villain")?, u("pot")?, u("to_call")?, &prizes)
                        .map_err(err)?,
                )
            }
        }
        other => return Err(err(format!("unknown theory calculation '{other}'"))),
    };
    to_json(&out.map_err(err)?)
}

#[pymodule]
fn riverlab_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(engine_version, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_hand, m)?)?;
    m.add_function(wrap_pyfunction!(compare_hands, m)?)?;
    m.add_function(wrap_pyfunction!(analyze_range, m)?)?;
    m.add_function(wrap_pyfunction!(equity, m)?)?;
    m.add_function(wrap_pyfunction!(validate_game_config, m)?)?;
    m.add_function(wrap_pyfunction!(default_game_config, m)?)?;
    m.add_function(wrap_pyfunction!(compute_rake, m)?)?;
    m.add_function(wrap_pyfunction!(stack_presets, m)?)?;
    m.add_function(wrap_pyfunction!(positions, m)?)?;
    m.add_function(wrap_pyfunction!(validate_solver_config, m)?)?;
    m.add_function(wrap_pyfunction!(validate_solve_spec, m)?)?;
    m.add_function(wrap_pyfunction!(replay_hand, m)?)?;
    m.add_function(wrap_pyfunction!(solver_backends, m)?)?;
    m.add_function(wrap_pyfunction!(solve_postflop, m)?)?;
    m.add_class::<CancelToken>()?;
    m.add_class::<SolvedGame>()?;
    m.add_function(wrap_pyfunction!(load_solved, m)?)?;
    m.add_function(wrap_pyfunction!(validate_preflop_spec, m)?)?;
    m.add_function(wrap_pyfunction!(solve_preflop, m)?)?;
    m.add_function(wrap_pyfunction!(load_preflop, m)?)?;
    m.add_class::<PreflopGame>()?;
    m.add_class::<PlaySession>()?;
    m.add_function(wrap_pyfunction!(theory, m)?)?;
    m.add_function(wrap_pyfunction!(validate_table_config, m)?)?;
    m.add_function(wrap_pyfunction!(play_report, m)?)?;
    m.add_function(wrap_pyfunction!(play_spot, m)?)?;
    m.add_function(wrap_pyfunction!(review_streets, m)?)?;
    m.add_function(wrap_pyfunction!(review_street_spec, m)?)?;
    m.add_function(wrap_pyfunction!(review_analyze, m)?)?;
    m.add_function(wrap_pyfunction!(push_fold_build_table, m)?)?;
    m.add_class::<PushFoldTrainer>()?;
    Ok(())
}
