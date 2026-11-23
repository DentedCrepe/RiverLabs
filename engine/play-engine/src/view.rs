//! What the table UI shows. Bots' cards appear only after a showdown they reached.

use crate::config::Format;
use crate::session::{Session, SessionStatus, street_name};
use poker_core::HoleCards;
use poker_core::game::{GameState, Phase};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SeatView {
    pub name: String,
    pub is_hero: bool,
    pub position: Option<String>,
    pub is_button: bool,
    /// Chips behind (during a hand) or total chips (between hands).
    pub stack: u64,
    pub street_commit: u64,
    pub folded: bool,
    pub all_in: bool,
    pub busted: bool,
    pub finish: Option<u8>,
    pub cards: Option<Vec<String>>,
    pub in_hand: bool,
    /// The seat's latest action this hand, e.g. "raises to 2.5bb".
    pub last_action: Option<String>,
    /// After a hand: cards of players who didn't show them, for an optional "show all hands".
    pub hidden_cards: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionFeedback {
    pub street: String,
    pub action: String,
    pub feedback: crate::judge::Feedback,
}

#[derive(Debug, Clone, Serialize)]
pub struct LegalView {
    pub fold: bool,
    pub check: bool,
    pub call: Option<u64>,
    pub bet: Option<(u64, u64)>,
    pub raise: Option<(u64, u64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub street: String,
    pub player: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableView {
    pub status: SessionStatus,
    pub format: String,
    /// "bb" (amounts are mbb) or "chips".
    pub units: String,
    pub big_blind: u64,
    pub small_blind: u64,
    pub ante: u64,
    pub level: Option<usize>,
    pub hand_no: u64,
    pub street: Option<String>,
    pub board: Vec<String>,
    pub pot: u64,
    pub seats: Vec<SeatView>,
    /// Index into `seats` of the player to act.
    pub to_act: Option<usize>,
    pub legal: Option<LegalView>,
    pub log: Vec<LogLine>,
    /// Last finished hand: your net result in chip units.
    pub last_hand_net: Option<i64>,
    pub hero_total_net: i64,
    pub notes: Vec<String>,
    /// Practice mode: the bots' cards are shown during hands.
    pub reveal: bool,
    /// Feedback on each of your decisions in the current (or just finished) hand.
    pub feedback: Vec<DecisionFeedback>,
    /// Result lines of the finished hand.
    pub summary: Vec<String>,
}

#[derive(Debug)]
pub struct HandRef<'a> {
    pub state: &'a GameState,
    pub seat_player: &'a [usize],
    pub hole: &'a [HoleCards],
    pub record: &'a crate::record::HandRecord,
}

fn amount(units_chips: bool, x: u64) -> String {
    if units_chips { x.to_string() } else { poker_core::Chips(x).to_string() }
}

/// Your own action, in the second person ("call", "raise to 7.5bb").
pub fn describe_yours(a: &poker_core::game::Action, chips: bool) -> String {
    use poker_core::game::Action::*;
    match a {
        Fold => "fold".into(),
        Check => "check".into(),
        Call => "call".into(),
        Bet { to } => format!("bet {}", amount(chips, to.mbb())),
        Raise { to } => format!("raise to {}", amount(chips, to.mbb())),
    }
}

pub fn describe(a: &poker_core::game::Action, chips: bool, all_in: bool) -> String {
    use poker_core::game::Action::*;
    let ai = if all_in { " (all-in)" } else { "" };
    match a {
        Fold => "folds".into(),
        Check => "checks".into(),
        Call => format!("calls{ai}"),
        Bet { to } => format!("bets {}{ai}", amount(chips, to.mbb())),
        Raise { to } => format!("raises to {}{ai}", amount(chips, to.mbb())),
    }
}

pub fn build(s: &Session, hand: Option<HandRef>, notes: &[String]) -> TableView {
    let chips = matches!(s.config().format, Format::SitAndGo { .. });
    let last = s.records().last();
    let hero_total_net: i64 = s.records().iter().map(|r| r.net[r.hero_seat()]).sum();
    let (format, (sb, bb, ante)) = match &s.config().format {
        Format::Cash { game, .. } => ("cash", (game.small_blind.mbb(), 1000, game.ante.mbb())),
        Format::FastFold { game } => ("fast_fold", (game.small_blind.mbb(), 1000, game.ante.mbb())),
        Format::SitAndGo { .. } => {
            let l = s.level_info().map(|(_, l)| l);
            ("sit_and_go", l.map_or((0, 0, 0), |l| (l.small_blind, l.big_blind, l.ante)))
        }
    };
    let mut seats: Vec<SeatView> = s
        .players
        .iter()
        .enumerate()
        .map(|(i, p)| SeatView {
            name: p.name.clone(),
            is_hero: i == 0,
            position: None,
            is_button: i == s.button(),
            stack: p.chips,
            street_commit: 0,
            folded: false,
            all_in: false,
            busted: p.busted,
            finish: p.finish,
            cards: None,
            in_hand: false,
            last_action: None,
            hidden_cards: None,
        })
        .collect();
    let mut view = TableView {
        status: s.status(),
        format: format.into(),
        units: if chips { "chips" } else { "bb" }.into(),
        big_blind: bb,
        small_blind: sb,
        ante,
        level: s.level_info().map(|(i, _)| i),
        hand_no: s.hands_played() + u64::from(s.status() == SessionStatus::AwaitingHero),
        street: None,
        board: vec![],
        pot: 0,
        seats: vec![],
        to_act: None,
        legal: None,
        log: vec![],
        last_hand_net: None,
        hero_total_net,
        notes: notes.to_vec(),
        reveal: s.reveal(),
        feedback: vec![],
        summary: vec![],
    };
    let feedback_of = |r: &crate::record::HandRecord| -> Vec<DecisionFeedback> {
        r.hero_decisions
            .iter()
            .filter_map(|d| {
                d.feedback.clone().map(|feedback| DecisionFeedback {
                    street: d.street.clone(),
                    action: describe_yours(&d.action, chips),
                    feedback,
                })
            })
            .collect()
    };
    let last_actions = |r: &crate::record::HandRecord| -> Vec<Option<String>> {
        (0..r.seats.len())
            .map(|k| r.actions.iter().rev().find(|a| a.seat == k).map(|a| describe(&a.action, chips, a.all_in)))
            .collect()
    };
    match hand {
        Some(h) => {
            let st = h.state;
            view.street = Some(street_name(st.street()).into());
            view.board = st.board().cards().iter().map(|c| c.to_string()).collect();
            view.pot = st.pot_total().mbb();
            view.big_blind = st.big_blind().mbb();
            for (k, &p) in h.seat_player.iter().enumerate() {
                let ps = &st.players()[k];
                let seat = &mut seats[p];
                seat.position = Some(ps.position.short_name().into());
                seat.stack = ps.stack.mbb();
                seat.street_commit = ps.street_commit.mbb();
                seat.folded = ps.folded;
                seat.all_in = ps.all_in;
                seat.in_hand = true;
                if p == 0 || s.reveal() {
                    seat.cards = Some(h.hole[k].cards().iter().map(|c| c.to_string()).collect());
                }
                seat.last_action = last_actions(h.record)[k].clone();
            }
            view.feedback = feedback_of(h.record);
            if let Phase::Betting { to_act } = st.phase() {
                view.to_act = Some(h.seat_player[to_act]);
                if h.seat_player[to_act] == 0 {
                    if let Ok(l) = st.legal_actions() {
                        view.legal = Some(LegalView {
                            fold: l.fold,
                            check: l.check,
                            call: l.call.map(|c| c.mbb()),
                            bet: l.bet.map(|(a, b)| (a.mbb(), b.mbb())),
                            raise: l.raise.map(|(a, b)| (a.mbb(), b.mbb())),
                        });
                    }
                }
            }
            view.log = h
                .record
                .actions
                .iter()
                .map(|r| LogLine {
                    street: r.street.clone(),
                    player: s.players[h.seat_player[r.seat]].name.clone(),
                    text: describe(&r.action, chips, r.all_in),
                })
                .collect();
        }
        None => {
            if let Some(r) = last {
                view.board = r.board.clone();
                view.last_hand_net = Some(r.net[r.hero_seat()]);
                view.feedback = feedback_of(r);
                view.summary = r.summary.clone();
                let acts = last_actions(r);
                view.log = r
                    .actions
                    .iter()
                    .map(|a| LogLine {
                        street: a.street.clone(),
                        player: r.seats[a.seat].name.clone(),
                        text: describe(&a.action, chips, a.all_in),
                    })
                    .collect();
                let heads_up = r.seats.len() == 2;
                for (k, rs) in r.seats.iter().enumerate() {
                    if let Some(seat) = seats.iter_mut().find(|x| x.name == rs.name) {
                        // Show the finished hand's button with its positions.
                        seat.is_button = rs.position == "BTN" || (heads_up && rs.position == "SB");
                        seat.position = Some(rs.position.clone());
                        seat.in_hand = true;
                        if rs.is_hero || (r.showdown && rs.shown) {
                            seat.cards = Some(rs.hole_cards.clone());
                        } else {
                            seat.hidden_cards = Some(rs.hole_cards.clone());
                        }
                        seat.last_action = acts[k].clone();
                        let folded =
                            r.actions.iter().any(|a| a.seat == k && a.action == poker_core::game::Action::Fold);
                        seat.folded = folded;
                    }
                }
            }
        }
    }
    view.seats = seats;
    view
}
