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
        "scope": "heads-up postflop (flop, turn or river root)",
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
    Ok(())
}
