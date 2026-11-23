//! Solver review of a whole hand: one solve per street, chained.
//!
//! - [`streets`] lists the streets after the flop where you made decisions, and whether each can
//!   be solved (heads-up when the street started).
//! - [`street_spec`] builds the solve for one street. The first solved street starts from your
//!   range in the bots' preflop model and the bot's tracked range; each later street starts from
//!   both players' ranges at the end of the previous solved street, following what actually
//!   happened (subgame re-solving).
//! - [`analyze`] follows the real line through a solved street and, at each of your decisions,
//!   reports the solver's mix for your exact hand, the EV of each action and the EV you lost.
//!
//! Every result is an approximate equilibrium of its street's tree; see `docs/play.md`.

use crate::config::{Format, PlayError, Result, TableConfig};
use crate::record::{HandRecord, RecordAction};
use crate::spot::{EXPORT_ITERATIONS, Observed, TREES, describe_tree, pct, round4, tree_at};
use poker_core::game::Action;
use poker_core::rake::RakeConfig;
use poker_core::{Board, Card, GameConfig};
use range_engine::{Combo, Range};
use serde::{Deserialize, Serialize};
use solver_gtopen::{PathStep, Solved};
use solver_interface::{SolveSpec, SolverConfig};

/// Weight given to your own hand when the previous street's solution never plays it this way,
/// so that its strategy can still be shown. Small enough not to move the equilibrium noticeably.
pub const MIN_HERO_WEIGHT: f64 = 0.0001;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewStreet {
    pub street: String,
    /// Indexes into `hero_decisions`.
    pub decisions: Vec<usize>,
    pub solvable: bool,
    /// Why a street cannot be solved.
    pub note: Option<String>,
}

/// Streets after the flop where you made decisions.
pub fn streets(record: &HandRecord) -> Vec<ReviewStreet> {
    ["flop", "turn", "river"]
        .iter()
        .filter_map(|&st| {
            let decisions: Vec<usize> =
                (0..record.hero_decisions.len()).filter(|&i| record.hero_decisions[i].street == st).collect();
            if decisions.is_empty() {
                return None;
            }
            let solvable = decisions.iter().all(|&i| record.hero_decisions[i].spot.is_some());
            Some(ReviewStreet {
                street: st.into(),
                note: (!solvable).then(|| {
                    "Not solvable: more than two players were in the hand on this street, or it was recorded \
                     before spots were saved. RiverLab solves heads-up spots only."
                        .into()
                }),
                decisions,
                solvable,
            })
        })
        .collect()
}

/// One action of the street's line, in solve units (mbb).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewStep {
    /// 0 = out of position, 1 = in position.
    pub player: u8,
    pub kind: String,
    pub to_mbb: u64,
    pub all_in: bool,
    /// Bets: % of the pot; raises: increment as % of the pot after calling (GTOpen's convention).
    pub pct: Option<u32>,
    /// For your actions: the index into `hero_decisions`.
    pub decision: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreetSpec {
    pub street: String,
    pub spec: SolveSpec,
    /// Every action on the street, in order.
    pub line: Vec<ReviewStep>,
    pub hero_player: u8,
    pub hero_combo: String,
    /// The next street's card, when the hand continued.
    pub next_card: Option<String>,
    pub notes: Vec<String>,
}

fn street_board_len(street: &str) -> usize {
    match street {
        "flop" => 3,
        "turn" => 4,
        _ => 5,
    }
}

fn hero_combo(record: &HandRecord) -> Result<Combo> {
    let cards = &record.seats[record.hero_seat()].hole_cards;
    let bad = || PlayError::Engine("invalid hole cards in the record".into());
    let a: Card = cards.first().ok_or_else(bad)?.parse().map_err(|_| bad())?;
    let b: Card = cards.get(1).ok_or_else(bad)?.parse().map_err(|_| bad())?;
    Combo::new(a, b).ok_or_else(bad)
}

/// Record chips to mbb (1bb = 1000): unchanged at cash tables, converted from tournament chips.
fn to_mbb(record: &HandRecord, x: u64, inexact: &mut bool) -> u64 {
    if !record.tournament {
        return x;
    }
    let bb = record.big_blind.max(1) as u128;
    let scaled = x as u128 * 1000;
    if scaled % bb != 0 {
        *inexact = true;
    }
    ((scaled + bb / 2) / bb) as u64
}

struct RawStep {
    hero: bool,
    kind: String,
    to: u64,
    all_in: bool,
    pct: Option<u32>,
    decision: Option<usize>,
}

/// The street's actions with sizes as GTOpen percentages, in record chip units.
fn street_line(record: &HandRecord, street: &str, hero: usize, start_pot: u64) -> Vec<RawStep> {
    // Your k-th action in the hand is hero_decisions[k].
    let mut hero_k = 0usize;
    let mut out = Vec::new();
    let mut commit: std::collections::HashMap<usize, u64> = Default::default();
    let mut pot = start_pot;
    for a in &record.actions {
        let is_hero = a.seat == hero;
        let k = is_hero.then_some(hero_k);
        if is_hero {
            hero_k += 1;
        }
        if a.street != street {
            continue;
        }
        let RecordAction { seat, action, street_commit, all_in, .. } = a;
        let mine = *commit.get(seat).unwrap_or(&0);
        let other = commit.iter().filter(|(s, _)| *s != seat).map(|(_, c)| *c).max().unwrap_or(0);
        let facing = other.saturating_sub(mine);
        let (kind, p) = match action {
            Action::Fold => ("fold", None),
            Action::Check => ("check", None),
            Action::Call => ("call", None),
            Action::Bet { to } => ("bet", pct(to.mbb(), pot)),
            Action::Raise { to } => ("raise", pct(to.mbb().saturating_sub(other), pot + facing)),
        };
        let to = if matches!(action, Action::Fold | Action::Check) { 0 } else { *street_commit };
        if *street_commit > mine {
            pot += street_commit - mine;
            commit.insert(*seat, *street_commit);
        }
        out.push(RawStep { hero: is_hero, kind: kind.to_string(), to, all_in: *all_in, pct: p, decision: k });
    }
    out
}

/// The solve for one street. `carried` is `[oop, ip]` range notation from the end of the previous
/// solved street; `None` starts from the preflop model (you) and the tracked range (the bot).
pub fn street_spec(
    record: &HandRecord,
    street: &str,
    cfg: &TableConfig,
    max_bytes: u64,
    carried: Option<[String; 2]>,
) -> Result<StreetSpec> {
    let info = streets(record)
        .into_iter()
        .find(|s| s.street == street && s.solvable)
        .ok_or_else(|| PlayError::State(format!("the {street} of hand {} cannot be solved", record.hand_no)))?;
    let spot = record.hero_decisions[info.decisions[0]].spot.as_ref().expect("solvable streets have spots");
    let hero = record.hero_seat();
    let combo = hero_combo(record)?;
    let mut notes = Vec::new();
    let mut inexact = false;
    let pot = to_mbb(record, spot.pot, &mut inexact);
    let stack = to_mbb(record, spot.effective_stack, &mut inexact);
    let hero_player: u8 = if spot.hero_oop { 0 } else { 1 };
    let player = |is_hero: bool| if is_hero { hero_player } else { 1 - hero_player };
    let line: Vec<ReviewStep> = street_line(record, street, hero, spot.pot)
        .into_iter()
        .map(|r| ReviewStep {
            player: player(r.hero),
            kind: r.kind,
            to_mbb: to_mbb(record, r.to, &mut inexact),
            all_in: r.all_in,
            pct: r.pct,
            decision: r.decision,
        })
        .collect();
    if record.tournament {
        notes.push(format!(
            "Tournament chips are converted to big blinds (big blind {} chips); chip EV, no ICM.",
            record.big_blind
        ));
        if inexact {
            notes.push("Some amounts were rounded to 0.001bb when converting chips to big blinds.".into());
        }
    }

    // Ranges.
    let mut ranges: [String; 2] = match carried {
        Some(r) => {
            notes.push(
                "Both ranges start where the previous street's solve ended, following what actually happened \
                 (subgame re-solving). They inherit that solve's bet sizes and approximations."
                    .into(),
            );
            r
        }
        None => {
            let mine = match &spot.hero_range {
                Some(r) => {
                    notes.push(
                        "Your range is your range at the flop in the bots' preflop model (an approximate model)."
                            .into(),
                    );
                    r.clone()
                }
                None => {
                    notes.push(
                        "This hand left the preflop model, so your starting range is unknown and set to every hand."
                            .into(),
                    );
                    Range::full().to_notation()
                }
            };
            notes
                .push("The bot's range is the range its rules actually played to this street, tracked exactly.".into());
            if spot.hero_oop { [mine, spot.opponent_range.clone()] } else { [spot.opponent_range.clone(), mine] }
        }
    };
    let h = hero_player as usize;
    let mut hero_range = Range::parse(&ranges[h]).map_err(|e| PlayError::Engine(e.to_string()))?;
    if hero_range.weight(combo) < MIN_HERO_WEIGHT {
        hero_range.set_weight(combo, MIN_HERO_WEIGHT).map_err(|e| PlayError::Engine(e.to_string()))?;
        ranges[h] = hero_range.to_notation();
        notes.push(format!(
            "The solution so far never reaches here with your hand ({combo}); it was added with a tiny weight \
             ({MIN_HERO_WEIGHT}) so its strategy can be shown."
        ));
    }

    let cards: Vec<Card> = spot
        .board
        .iter()
        .map(|c| c.parse())
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| PlayError::Engine(format!("{e}")))?;
    let board = Board::new(cards).map_err(|e| PlayError::Engine(e.to_string()))?;
    let s = spot.board.len() - 3;
    let game = match &cfg.format {
        Format::Cash { game, .. } | Format::FastFold { game } => game.clone(),
        Format::SitAndGo { .. } => GameConfig { rake: RakeConfig::none(), ..GameConfig::default() },
    };
    let mut observed = Observed { bets: [vec![], vec![]], raises: [vec![], vec![]], raise_count: 0, all_in: false };
    for st in &line {
        let p = st.player as usize;
        observed.all_in |= st.all_in && (st.kind == "bet" || st.kind == "raise");
        match (st.kind.as_str(), st.pct) {
            ("bet", Some(x)) => observed.bets[p].push(x),
            ("raise", Some(x)) => {
                observed.raises[p].push(x);
                observed.raise_count += 1;
            }
            _ => {}
        }
    }
    let mut solver = SolverConfig::default();
    solver.stopping.max_iterations = EXPORT_ITERATIONS;
    solver.stopping.exploitability_check_interval = 25;
    let mut spec = SolveSpec {
        game,
        ranges: ranges.to_vec(),
        board,
        starting_pot_mbb: pot,
        effective_stack_mbb: stack,
        tree: tree_at(&observed, s, TREES[0]),
        solver,
    };
    let mut chosen = (0, 0);
    for (k, shape) in TREES.iter().enumerate() {
        spec.tree = tree_at(&observed, s, *shape);
        spec.validate().map_err(|e| PlayError::Engine(e.to_string()))?;
        let needed = solver_gtopen::memory_needed(&spec, solver_gtopen::Limits::default().max_nodes)
            .map_err(|e| PlayError::Engine(e.to_string()))?;
        chosen = (k, needed);
        if needed <= max_bytes {
            break;
        }
    }
    notes.insert(0, describe_tree(TREES[chosen.0], street, s));
    if chosen.1 > max_bytes {
        notes.insert(
            0,
            format!(
                "Even the smallest tree needs {:.0} MB, above the solver's limit of {:.0} MB.",
                chosen.1 as f64 / 1_048_576.0,
                max_bytes as f64 / 1_048_576.0
            ),
        );
    }
    let next = street_board_len(street);
    Ok(StreetSpec {
        street: street.into(),
        spec,
        line,
        hero_player,
        hero_combo: combo.to_string(),
        next_card: record.board.get(next).filter(|_| next < 5).cloned(),
        notes,
    })
}

/// What the solver says about one of your decisions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionReview {
    /// Index into `hero_decisions`.
    pub decision: usize,
    /// The actions taken before it on this street (tree labels).
    pub before: Vec<String>,
    pub actions: Vec<String>,
    /// The solver's mix for your exact hand.
    pub strategy: Option<Vec<f64>>,
    /// EV of each action for your hand (bb, from the start of the street, net of your half of the
    /// starting pot), against the range that reaches this point, both players following the
    /// solution afterwards.
    pub action_evs_bb: Option<Vec<f64>>,
    /// The tree action your action was matched to (nearest size of the same kind).
    pub taken: usize,
    pub best: Option<usize>,
    /// Best action's EV minus yours (bb), never negative.
    pub ev_loss_bb: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreetReview {
    pub street: String,
    pub decisions: Vec<DecisionReview>,
    /// `[oop, ip]` range notation at the start of the next street, if the hand went on.
    pub next_ranges: Option<[String; 2]>,
    /// The whole line was found in the tree.
    pub followed: bool,
    pub notes: Vec<String>,
}

/// The tree action matching a real action: same kind, nearest "to" amount ("bet 6.6bb").
pub fn match_action(labels: &[String], kind: &str, to_mbb: u64) -> Option<usize> {
    let kind_of = |l: &str| {
        if l.starts_with("raise") { "raise".to_string() } else { l.split(' ').next().unwrap_or("").to_string() }
    };
    let amount = |l: &str| l.rsplit(' ').next().and_then(|a| a.strip_suffix("bb")).and_then(|a| a.parse::<f64>().ok());
    labels
        .iter()
        .enumerate()
        .filter(|(_, l)| kind_of(l) == kind)
        .min_by(|(_, a), (_, b)| {
            let d = |l: &str| amount(l).map_or(0.0, |x| (x * 1000.0 - to_mbb as f64).abs());
            d(a).total_cmp(&d(b))
        })
        .map(|(i, _)| i)
}

/// Follows the street's real line through its solution.
pub fn analyze(solved: &Solved, s: &StreetSpec) -> Result<StreetReview> {
    let err = |e: solver_gtopen::AdapterError| PlayError::Engine(e.to_string());
    let combo = Combo::from(
        s.hero_combo.parse::<poker_core::HoleCards>().map_err(|_| PlayError::Engine("invalid hero combo".into()))?,
    );
    let hero = s.hero_player as usize;
    let mut path: Vec<PathStep> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    let mut decisions = Vec::new();
    let mut notes = Vec::new();
    for step in &s.line {
        let node = solved.hand_at(&path, hero, combo).map_err(err)?;
        if node.kind != "decision" || node.player != Some(step.player as usize) {
            notes.push(format!("The line left the solved tree before '{}' (a {} node).", step.kind, node.kind));
            return Ok(StreetReview { street: s.street.clone(), decisions, next_ranges: None, followed: false, notes });
        }
        let Some(a) = match_action(&node.actions, &step.kind, step.to_mbb) else {
            notes.push(format!("The tree has no '{}' at this point, so the line could not be followed.", step.kind));
            return Ok(StreetReview { street: s.street.clone(), decisions, next_ranges: None, followed: false, notes });
        };
        if let Some(d) = step.decision.filter(|_| step.player as usize == hero) {
            let evs = node.action_evs_mbb.as_ref().map(|v| v.iter().map(|x| x / 1000.0).collect::<Vec<f64>>());
            let best = evs.as_ref().and_then(|v| (0..v.len()).max_by(|&i, &j| v[i].total_cmp(&v[j])));
            decisions.push(DecisionReview {
                decision: d,
                before: labels.clone(),
                actions: node.actions.clone(),
                strategy: node.strategy.clone(),
                ev_loss_bb: evs.as_ref().zip(best).map(|(v, b)| (v[b] - v[a]).max(0.0)),
                action_evs_bb: evs,
                taken: a,
                best,
            });
        }
        labels.push(node.actions[a].clone());
        path.push(PathStep::Action(a));
    }
    // Ranges where the next street starts.
    let mut next_ranges = None;
    if let Some(card) = &s.next_card {
        let c: Card = card.parse().map_err(|_| PlayError::Engine("invalid next card".into()))?;
        let mut p = path.clone();
        p.push(PathStep::Card(c));
        match solved.hand_at(&p, hero, combo) {
            Ok(node) => {
                let note = |r: &[(Combo, f64)]| {
                    let mut range = Range::empty();
                    for (combo, w) in r {
                        let _ = range.set_weight(*combo, round4(w.min(1.0)));
                    }
                    (!range.is_empty()).then(|| range.to_notation())
                };
                match (note(&node.ranges[0]), note(&node.ranges[1])) {
                    (Some(oop), Some(ip)) => next_ranges = Some([oop, ip]),
                    _ => notes.push(
                        "One player's range is empty after this line under the solution, so the next street starts \
                         again from the tracked ranges."
                            .into(),
                    ),
                }
            }
            Err(e) => notes.push(format!("The next street could not be reached in this tree ({e}).")),
        }
    }
    Ok(StreetReview { street: s.street.clone(), decisions, next_ranges, followed: true, notes })
}
