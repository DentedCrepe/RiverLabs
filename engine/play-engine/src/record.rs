//! Hand records: what happened, plus what you faced at each of your decisions.

use crate::config::Difficulty;
use poker_core::game::{Action, GameState, Settlement};
use poker_core::{Board, Card};
use range_engine::Combo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordSeat {
    pub name: String,
    pub difficulty: Option<Difficulty>,
    pub position: String,
    pub starting_stack: u64,
    /// Every player's cards, kept for the report's all-in calculation. The UI shows a bot's
    /// cards only when `shown` (they reached showdown).
    pub hole_cards: Vec<String>,
    pub shown: bool,
    pub is_hero: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordAction {
    /// Index into `seats`.
    pub seat: usize,
    pub street: String,
    pub action: Action,
    pub added: u64,
    pub street_commit: u64,
    pub all_in: bool,
}

/// The preflop model's view of one of your decisions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCheck {
    /// "unopened", "vs_limp", "vs_open", "vs_3bet", "vs_4bet".
    pub spot: String,
    pub class: String,
    pub actions: Vec<String>,
    /// Model frequency of each action for your hand class.
    pub frequencies: Vec<f64>,
    /// The model action your action was mapped to (`None`: not in the model tree).
    pub taken: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeroDecision {
    pub street: String,
    pub action: Action,
    pub to_call: u64,
    /// Pot before your action.
    pub pot: u64,
    /// to_call / (pot + to_call): the equity a call needs to break even, ignoring later betting.
    pub required_equity: Option<f64>,
    /// Your equity against the live bots' tracked ranges at this point (Monte Carlo).
    pub equity_vs_ranges: Option<f64>,
    pub opponents: usize,
    pub model: Option<ModelCheck>,
    /// Calling would put you all-in (no later betting, so pot odds are the whole story).
    pub all_in_call: bool,
    /// Shown to you straight after the decision (same rules as the report; see judge.rs).
    #[serde(default)]
    pub feedback: Option<crate::judge::Feedback>,
    /// Heads-up after the flop: what "solve this spot" needs (see spot.rs).
    #[serde(default)]
    pub spot: Option<crate::spot::SpotInfo>,
    /// Sit & go, heads-up, and calling ends the betting (someone is all-in): the call valued
    /// with ICM for the tournament's payouts (see docs/play.md).
    #[serde(default)]
    pub icm: Option<IcmCheck>,
}

/// An all-in call in a sit & go valued with ICM (Malmuth–Harville): your share of the
/// remaining prize money if you fold, call and win, or call and lose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IcmCheck {
    /// Percent of the prize pool for 1st, 2nd, ...
    pub payouts: Vec<u32>,
    pub fold: f64,
    pub win: f64,
    pub lose: f64,
    /// (fold − lose) / (win − lose): the equity a call needs under ICM.
    pub required_equity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllInSnapshot {
    pub street: String,
    pub board: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandRecord {
    pub hand_no: u64,
    pub level: Option<usize>,
    /// Amounts are tournament chips (otherwise mbb).
    pub tournament: bool,
    pub small_blind: u64,
    pub big_blind: u64,
    pub ante: u64,
    /// In preflop action order (UTG ... SB, BB).
    pub seats: Vec<RecordSeat>,
    pub board: Vec<String>,
    pub actions: Vec<RecordAction>,
    /// Per seat, in chip units.
    pub net: Vec<i64>,
    pub showdown: bool,
    pub hero_decisions: Vec<HeroDecision>,
    /// Set when the money went in before the river with no betting left.
    pub all_in: Option<AllInSnapshot>,
    /// Your expected net result at the moment of the all-in (exact equity over the remaining
    /// board), in chip units. Compare with `net` to see how lucky the run-out was.
    pub hero_all_in_ev: Option<f64>,
    /// Stack bucket of the bots' preflop model, when one was used.
    pub model_stack_bb: Option<u64>,
    pub notes: Vec<String>,
    /// The bots' cards were revealed to you during this hand (practice mode).
    #[serde(default)]
    pub revealed: bool,
    /// Who won what, e.g. "Bot 2 wins 12.5bb with Two Pair, Kings and Sevens".
    #[serde(default)]
    pub summary: Vec<String>,
}

impl HandRecord {
    pub fn hero_seat(&self) -> usize {
        self.seats.iter().position(|s| s.is_hero).expect("hero is seated")
    }
}

/// Preflop spot name for the player to act, from the raises so far.
pub fn spot_name(state: &GameState, seat: usize) -> String {
    let raises =
        state.history().iter().filter(|r| matches!(r.action, Action::Raise { .. } | Action::Bet { .. })).count();
    let limps = state.history().iter().filter(|r| r.action == Action::Call).count();
    let is_bb = seat + 1 == state.players().len();
    match raises {
        0 if limps == 0 => "unopened",
        0 if is_bb => "bb_option",
        0 => "vs_limp",
        1 => "vs_open",
        2 => "vs_3bet",
        _ => "vs_4bet",
    }
    .into()
}

fn combo(cards: &[String]) -> Option<Combo> {
    let a: Card = cards.first()?.parse().ok()?;
    let b: Card = cards.get(1)?.parse().ok()?;
    Combo::new(a, b)
}

/// Your all-in expectation: for each pot you are eligible for, your exact equity among its
/// eligible players over the remaining board, times the pot after rake; minus what you put in.
pub fn all_in_ev(record: &HandRecord, snap: &AllInSnapshot, settlement: &Settlement) -> Option<f64> {
    let hero = record.hero_seat();
    let board: Vec<Card> = snap.board.iter().filter_map(|c| c.parse().ok()).collect();
    let board = Board::new(board).ok()?;
    let mut expected = 0.0;
    for pot in &settlement.pots {
        if !pot.eligible.contains(&hero) {
            continue;
        }
        let net = (pot.amount.mbb() - pot.rake.mbb()) as f64;
        if pot.eligible.len() == 1 {
            expected += net;
            continue;
        }
        let ranges: Option<Vec<_>> = pot
            .eligible
            .iter()
            .map(|&s| combo(&record.seats[s].hole_cards).map(equity_engine::single_combo_range))
            .collect();
        let r = equity_engine::exact(&ranges?, &board, poker_core::CardSet::EMPTY, equity_engine::DEFAULT_EXACT_BUDGET)
            .ok()?;
        let k = pot.eligible.iter().position(|&s| s == hero)?;
        expected += r.equity[k] * net;
    }
    let committed = settlement.payouts[hero].mbb() as i64 - record.net[hero];
    Some(expected - committed as f64)
}
