//! "Solve this spot": turns one of your heads-up postflop decisions into a postflop solve.
//!
//! At each decision with exactly two players left after the flop, [`capture`] stores what a
//! solve needs: the street-start pot and effective stack, the board, the street's actions so
//! far (with their sizes as percentages of the pot, in GTOpen's convention), the bot's tracked
//! range and your range from the bots' preflop model. [`export`] builds a [`SolveSpec`] whose
//! tree contains the sizes actually used, so the solved tree can be followed to your decision.
//!
//! The export is a starting point, not a verdict: every approximation is listed in `notes`.

use crate::config::{Format, PlayError, Result, TableConfig};
use crate::record::HandRecord;
use poker_core::game::{Action, GameState};
use poker_core::rake::RakeConfig;
use poker_core::{Board, Card, GameConfig, Position};
use range_engine::{Combo, HandClass, Range};
use serde::{Deserialize, Serialize};
use solver_interface::{PlayerSizing, SolveSpec, SolverConfig, TreeAbstraction};

/// Default bet sizes (% of pot) added to every street of an exported tree.
pub const DEFAULT_BETS: [u32; 2] = [33, 75];
/// Default raise size (% of the pot after calling) on the decision street.
pub const DEFAULT_RAISE: u32 = 100;
/// Iteration cap for exported solves (they stop earlier at 0.5% of the pot).
pub const EXPORT_ITERATIONS: u64 = 500;

/// One action on the decision street, in record chip units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpotStep {
    pub hero: bool,
    /// "fold", "check", "call", "bet" or "raise".
    pub kind: String,
    /// Street total after the action (0 for fold and check).
    pub to: u64,
    /// Bets: % of the pot before the bet. Raises: the raise increment as a % of the pot after
    /// calling (GTOpen's convention). Rounded to a whole percent.
    pub pct: Option<u32>,
    pub all_in: bool,
}

/// What a solve of one of your decisions needs, stored with the decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpotInfo {
    pub street: String,
    pub board: Vec<String>,
    /// Pot at the start of the street, record chip units.
    pub pot: u64,
    /// Smaller of the two stacks at the start of the street, record chip units.
    pub effective_stack: u64,
    /// You act first on this street.
    pub hero_oop: bool,
    /// Record seat of the bot.
    pub opponent: usize,
    /// The street's actions before your decision.
    pub line: Vec<SpotStep>,
    pub hero_action: SpotStep,
    /// The bot's tracked range when the street was dealt, before its actions on this street
    /// (exact for its policy; weights rounded to 4 decimals).
    pub opponent_range: String,
    /// Your range from the bots' preflop model at the flop (`None`: the hand left the model).
    pub hero_range: Option<String>,
}

/// One step of the exported line, in solve units (mbb).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportStep {
    /// 0 = out of position (acts first), 1 = in position.
    pub player: u8,
    pub kind: String,
    pub to_mbb: u64,
    pub all_in: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpotExport {
    pub spec: SolveSpec,
    /// The street's actions before your decision; follow them from the root.
    pub line: Vec<ExportStep>,
    pub hero_action: ExportStep,
    pub hero_player: u8,
    pub hero_cards: Vec<String>,
    pub hero_class: String,
    pub street: String,
    pub notes: Vec<String>,
}

pub(crate) fn round4(w: f64) -> f64 {
    (w.clamp(0.0, 1.0) * 10_000.0).round() / 10_000.0
}

/// Range notation from per-combo weights, rounded to 4 decimals.
pub fn weights_notation(weights: &[f64]) -> Option<String> {
    let r = Range::from_weights(weights.iter().map(|&w| round4(w)).collect()).ok()?;
    (!r.is_empty()).then(|| r.to_notation())
}

/// Range notation from class fractions (`label`, fraction of the class's combos).
pub fn class_notation<'a>(classes: impl IntoIterator<Item = (&'a str, f64)>) -> Option<String> {
    let mut r = Range::empty();
    for (label, f) in classes {
        let class: HandClass = label.parse().ok()?;
        r.set_class(class, round4(f)).ok()?;
    }
    (!r.is_empty()).then(|| r.to_notation())
}

pub(crate) fn pct(num: u64, den: u64) -> Option<u32> {
    (den > 0).then(|| ((num as f64 * 100.0 / den as f64).round() as u32).clamp(1, 10_000))
}

/// The spot for the player `seat` about to take `action` (heads-up, after the flop), or `None`.
pub fn capture(
    state: &GameState,
    seat: usize,
    action: Action,
    opponent_weights: impl Fn(usize) -> Vec<f64>,
    hero_range: Option<String>,
) -> Option<SpotInfo> {
    let board = state.board().cards();
    if board.len() < 3 {
        return None;
    }
    let players = state.players();
    let live: Vec<usize> = (0..players.len()).filter(|&k| !players[k].folded).collect();
    if live.len() != 2 || !live.contains(&seat) {
        return None;
    }
    let opp = live.into_iter().find(|&k| k != seat)?;
    let street = state.street();
    let street_actions: Vec<_> = state.history().iter().filter(|r| r.street == street).collect();
    // Another player acting on this street (then folding) would leave dead money mid-street.
    if street_actions.iter().any(|r| r.seat != seat && r.seat != opp) {
        return None;
    }
    let committed: u64 = players.iter().map(|p| p.street_commit.mbb()).sum();
    let start_pot = state.pot_total().mbb() - committed;
    let start_stack = |k: usize| players[k].stack.mbb() + players[k].street_commit.mbb();
    let effective_stack = start_stack(seat).min(start_stack(opp));

    let mut commit = [0u64; 2]; // [hero, opponent]
    let mut pot = start_pot;
    let mut step = |is_hero: bool, a: Action, all_in: bool, added: Option<u64>| -> SpotStep {
        let (me, other) = if is_hero { (0, 1) } else { (1, 0) };
        let facing = commit[other].saturating_sub(commit[me]);
        let (kind, to, p) = match a {
            Action::Fold => ("fold", 0, None),
            Action::Check => ("check", 0, None),
            Action::Call => ("call", commit[other], None),
            Action::Bet { to } => ("bet", to.mbb(), pct(to.mbb(), pot)),
            Action::Raise { to } => ("raise", to.mbb(), pct(to.mbb().saturating_sub(commit[other]), pot + facing)),
        };
        let to = if kind == "call" { commit[other].min(commit[me] + added.unwrap_or(facing)) } else { to };
        if to > commit[me] {
            pot += to - commit[me];
            commit[me] = to;
        }
        SpotStep { hero: is_hero, kind: kind.into(), to, pct: p, all_in }
    };
    let line: Vec<SpotStep> =
        street_actions.iter().map(|r| step(r.seat == seat, r.action, r.all_in, Some(r.added.mbb()))).collect();
    let hero_all_in = match action {
        Action::Bet { to } | Action::Raise { to } => to.mbb() == start_stack(seat),
        Action::Call => state.legal_actions().ok().and_then(|l| l.call).is_some_and(|c| c == players[seat].stack),
        _ => false,
    };
    let hero_action = step(true, action, hero_all_in, None);

    let order = Position::postflop_order(players.len());
    let rank = |k: usize| order.iter().position(|&s| s == k).unwrap_or(usize::MAX);
    Some(SpotInfo {
        street: crate::session::street_name(street).into(),
        board: board.iter().map(|c| c.to_string()).collect(),
        pot: start_pot,
        effective_stack,
        hero_oop: rank(seat) < rank(opp),
        opponent: opp,
        line,
        hero_action,
        opponent_range: weights_notation(&opponent_weights(opp))?,
        hero_range,
    })
}

/// How rich an exported tree is. The sizes used in the hand are always added on its street.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TreeShape {
    /// Default bets on the decision street.
    bets: &'static [u32],
    /// Default raise on the decision street.
    raise: bool,
    /// Bets on later streets (never raises).
    later: &'static [u32],
}

/// From richest to leanest; the first that fits in memory is used.
pub(crate) const TREES: [TreeShape; 4] = [
    TreeShape { bets: &DEFAULT_BETS, raise: true, later: &DEFAULT_BETS },
    TreeShape { bets: &DEFAULT_BETS, raise: true, later: &[75] },
    TreeShape { bets: &[75], raise: false, later: &[75] },
    TreeShape { bets: &[], raise: false, later: &[75] },
];

/// Bet and raise sizes used on the decision street, per solver player.
pub(crate) struct Observed {
    pub(crate) bets: [Vec<u32>; 2],
    pub(crate) raises: [Vec<u32>; 2],
    pub(crate) raise_count: u8,
    pub(crate) all_in: bool,
}

fn observed_sizes(spot: &SpotInfo, player: impl Fn(bool) -> u8) -> Observed {
    let mut o = Observed { bets: [vec![], vec![]], raises: [vec![], vec![]], raise_count: 0, all_in: false };
    for step in spot.line.iter().chain([&spot.hero_action]) {
        let p = player(step.hero) as usize;
        o.all_in |= step.all_in && (step.kind == "bet" || step.kind == "raise");
        match (step.kind.as_str(), step.pct) {
            ("bet", Some(x)) => o.bets[p].push(x),
            ("raise", Some(x)) => {
                o.raises[p].push(x);
                o.raise_count += 1;
            }
            _ => {}
        }
    }
    o
}

pub(crate) fn tree_at(o: &Observed, s: usize, shape: TreeShape) -> TreeAbstraction {
    let mut sizing = [PlayerSizing::default(), PlayerSizing::default()];
    for (p, z) in sizing.iter_mut().enumerate() {
        let mut bets: Vec<u32> = shape.bets.iter().copied().chain(o.bets[p].iter().copied()).collect();
        if bets.is_empty() {
            bets.push(75); // leanest tree: still allow a bet where none was made
        }
        let mut raises = o.raises[p].clone();
        if shape.raise {
            raises.push(DEFAULT_RAISE);
        }
        for v in [&mut bets, &mut raises] {
            v.sort_unstable();
            v.dedup();
        }
        z.bet_pct_pot[s] = bets;
        z.raise_pct_pot[s] = raises;
        for st in s + 1..3 {
            z.bet_pct_pot[st] = shape.later.to_vec();
        }
    }
    let [oop, ip] = sizing;
    TreeAbstraction {
        oop,
        ip,
        max_raises_per_street: o.raise_count.max(1),
        // Snap the observed all-ins (rounded to whole percentages) back to all-in.
        all_in_threshold_pct_stack: if o.all_in { 98 } else { 100 },
        add_all_in: false,
    }
}

pub(crate) fn describe_tree(shape: TreeShape, street: &str, s: usize) -> String {
    let pcts = |v: &[u32]| v.iter().map(|b| format!("{b}%")).collect::<Vec<_>>().join(" and ");
    let mut text = if shape.bets.is_empty() {
        format!("Tree: on the {street}, only the sizes used in the hand (or a 75% bet where none was made)")
    } else {
        format!("Tree: on the {street}, bets of {} of the pot", pcts(shape.bets))
    };
    if shape.raise {
        text += &format!(", raises of {DEFAULT_RAISE}% of the pot after calling");
    }
    if !shape.bets.is_empty() {
        text += ", plus the sizes used in the hand";
    }
    text += " (rounded to whole percentages).";
    if s < 2 {
        text += &format!(" Later streets: bets of {} only, no raises.", pcts(shape.later));
    }
    text
}

/// Builds the solve for decision `index` of `record` (an index into `hero_decisions`).
/// `max_bytes`: the solver's memory limit; the export picks the richest tree that fits.
pub fn export(record: &HandRecord, index: usize, cfg: &TableConfig, max_bytes: u64) -> Result<SpotExport> {
    let d = record
        .hero_decisions
        .get(index)
        .ok_or_else(|| PlayError::State(format!("hand {} has no decision {index}", record.hand_no)))?;
    let spot = d.spot.as_ref().ok_or_else(|| {
        PlayError::State(
            "only heads-up decisions after the flop can be solved (this one was preflop, multiway, or recorded \
             before spots were saved)"
                .into(),
        )
    })?;
    let mut notes = Vec::new();
    let game = match &cfg.format {
        Format::Cash { game, .. } | Format::FastFold { game } => game.clone(),
        Format::SitAndGo { .. } => GameConfig { rake: RakeConfig::none(), ..GameConfig::default() },
    };
    // Tournament chips -> mbb (1bb = 1000), exact when the big blind divides evenly.
    let bb = record.big_blind.max(1);
    let mut inexact = false;
    let mut to_mbb = |x: u64| -> u64 {
        if !record.tournament {
            return x;
        }
        let scaled = x as u128 * 1000;
        if scaled % bb as u128 != 0 {
            inexact = true;
        }
        ((scaled + bb as u128 / 2) / bb as u128) as u64
    };
    let pot = to_mbb(spot.pot);
    let stack = to_mbb(spot.effective_stack);
    let hero_player: u8 = if spot.hero_oop { 0 } else { 1 };
    let player = |hero: bool| if hero { hero_player } else { 1 - hero_player };
    let mut conv = |s: &SpotStep| ExportStep {
        player: player(s.hero),
        kind: s.kind.clone(),
        to_mbb: to_mbb(s.to),
        all_in: s.all_in,
    };
    let line: Vec<ExportStep> = spot.line.iter().map(&mut conv).collect();
    let hero_action = conv(&spot.hero_action);
    if record.tournament {
        notes.push(format!(
            "Tournament chips are converted to big blinds (big blind {} chips); chip EV, no ICM.",
            record.big_blind
        ));
        if inexact {
            notes.push("Some amounts were rounded to 0.001bb when converting chips to big blinds.".into());
        }
    }

    let cards: Vec<Card> = spot
        .board
        .iter()
        .map(|c| c.parse())
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| PlayError::Engine(format!("{e}")))?;
    let board = Board::new(cards).map_err(|e| PlayError::Engine(e.to_string()))?;
    let s = spot.board.len() - 3;

    let observed = observed_sizes(spot, player);
    let hero_range = match &spot.hero_range {
        Some(r) => {
            notes.push(
                "Your range is your range at the flop in the bots' preflop model (an approximate model, with your real \
                 bets mapped to the nearest model sizes). It is not narrowed by your actions after the flop."
                    .into(),
            );
            r.clone()
        }
        None => {
            notes.push(
                "This hand left the preflop model, so your range is unknown: it is set to every hand. Edit it before \
                 solving."
                    .into(),
            );
            Range::full().to_notation()
        }
    };
    notes.push(
        "The bot's range is the range its rules actually played this way (tracked exactly; weights rounded to 4 \
         decimals). The solve finds an approximate equilibrium between the two ranges, not the bot's real strategy."
            .into(),
    );
    let ranges = if spot.hero_oop {
        vec![hero_range, spot.opponent_range.clone()]
    } else {
        vec![spot.opponent_range.clone(), hero_range]
    };
    let mut solver = SolverConfig::default();
    solver.stopping.max_iterations = EXPORT_ITERATIONS;
    solver.stopping.exploitability_check_interval = 25;
    let mut spec = SolveSpec {
        game,
        ranges,
        board,
        starting_pot_mbb: pot,
        effective_stack_mbb: stack,
        tree: tree_at(&observed, s, TREES[0]),
        solver,
        locks: Vec::new(),
    };
    // The richest tree that fits in the solver's memory limit (the hand's sizes are always kept).
    let mut chosen = None;
    for (k, shape) in TREES.iter().enumerate() {
        spec.tree = tree_at(&observed, s, *shape);
        spec.validate().map_err(|e| PlayError::Engine(e.to_string()))?;
        let needed = solver_gtopen::memory_needed(&spec, solver_gtopen::Limits::default().max_nodes)
            .map_err(|e| PlayError::Engine(e.to_string()))?;
        chosen = Some((k, needed));
        if needed <= max_bytes {
            break;
        }
    }
    let (k, needed) = chosen.expect("at least one tree shape");
    notes.insert(0, describe_tree(TREES[k], &spot.street, s));
    let mb = |b: u64| b as f64 / 1_048_576.0;
    if needed > max_bytes {
        notes.insert(
            0,
            format!(
                "Even the smallest tree needs {:.0} MB, above this RiverLab's solver limit of {:.0} MB: narrow the \
                 ranges before solving.",
                mb(needed),
                mb(max_bytes)
            ),
        );
    } else if k > 0 {
        notes.insert(
            0,
            format!(
                "Bet sizes were reduced so the solve fits in memory ({:.0} MB of {:.0} MB allowed).",
                mb(needed),
                mb(max_bytes)
            ),
        );
    }

    let hero = record.hero_seat();
    let hero_cards = record.seats[hero].hole_cards.clone();
    let hero_class = {
        let a: Card = hero_cards[0].parse().map_err(|e| PlayError::Engine(format!("{e}")))?;
        let b: Card = hero_cards[1].parse().map_err(|e| PlayError::Engine(format!("{e}")))?;
        HandClass::of(Combo::new(a, b).ok_or_else(|| PlayError::Engine("invalid hole cards".into()))?).to_string()
    };
    Ok(SpotExport { spec, line, hero_action, hero_player, hero_cards, hero_class, street: spot.street.clone(), notes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use poker_core::Chips;

    #[test]
    fn sizes_follow_gtopen_conventions() {
        // Heads-up, 100bb: SB limps, BB checks; flop pot 2bb.
        let mut g =
            GameState::with_stacks(GameConfig { players: 2, ..GameConfig::default() }, &[Chips(100_000); 2]).unwrap();
        g.apply(Action::Call).unwrap();
        g.apply(Action::Check).unwrap();
        let flop: Vec<Card> = ["Ks", "7d", "2c"].iter().map(|c| c.parse().unwrap()).collect();
        g.deal_board(&flop).unwrap();
        // BB (seat 1) acts first after the flop: bets 1.5bb (75%); SB raises to 6bb.
        g.apply(Action::Bet { to: Chips(1_500) }).unwrap();
        g.apply(Action::Raise { to: Chips(6_000) }).unwrap();
        let spot = capture(&g, 1, Action::Call, |_| vec![1.0; range_engine::COMBO_COUNT], None).unwrap();
        assert!(spot.hero_oop);
        assert_eq!((spot.pot, spot.effective_stack), (2_000, 99_000));
        let pcts: Vec<_> = spot.line.iter().map(|s| (s.kind.as_str(), s.to, s.pct)).collect();
        // The raise adds 4.5bb over a pot of 5bb after calling: 90%.
        assert_eq!(pcts, [("bet", 1_500, Some(75)), ("raise", 6_000, Some(90))]);
        assert_eq!((spot.hero_action.kind.as_str(), spot.hero_action.to), ("call", 6_000));
        assert_eq!(spot.line.iter().map(|s| s.hero).collect::<Vec<_>>(), [true, false]);
        // Preflop decisions have no spot.
        let pre =
            GameState::with_stacks(GameConfig { players: 2, ..GameConfig::default() }, &[Chips(100_000); 2]).unwrap();
        assert!(capture(&pre, 0, Action::Call, |_| vec![1.0; range_engine::COMBO_COUNT], None).is_none());
    }
}
