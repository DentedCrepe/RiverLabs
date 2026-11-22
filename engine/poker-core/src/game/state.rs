use super::action::{Action, ActionRecord, LegalActions};
use super::pots::{Pot, compute_pots};
use crate::card::Card;
use crate::cardset::CardSet;
use crate::chips::Chips;
use crate::config::GameConfig;
use crate::error::{PokerError, Result};
use crate::holding::{Board, HoleCards, Street};
use crate::position::Position;
use crate::rake::RakeResult;
use crate::stack::effective_stack;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    pub position: Position,
    pub starting_stack: Chips,
    /// Chips behind (not yet committed).
    pub stack: Chips,
    /// Committed on the current street (blinds count; antes do not).
    pub street_commit: Chips,
    /// Committed over the whole hand, including antes.
    pub total_commit: Chips,
    pub folded: bool,
    pub all_in: bool,
    /// Has voluntarily acted on the current street.
    pub acted: bool,
    /// After acting, the player may raise again only if the bet reaches this level
    /// (the minimum raise-to at the time they acted). `None` = unrestricted.
    pub reraise_threshold: Option<Chips>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum Phase {
    /// Waiting for `to_act` to choose an action.
    Betting { to_act: usize },
    /// Betting on the previous street is closed; the given street's cards must be dealt.
    AwaitingBoard { street: Street },
    /// All betting is complete with two or more live players.
    Showdown,
    /// Everyone else folded.
    Uncontested { winner: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PotResult {
    pub amount: Chips,
    pub rake: Chips,
    pub eligible: Vec<usize>,
    pub winners: Vec<usize>,
    /// Amount paid to each winner, aligned with `winners`.
    pub shares: Vec<Chips>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settlement {
    pub pots: Vec<PotResult>,
    pub rake: RakeResult,
    /// Total won per seat.
    pub payouts: Vec<Chips>,
    pub final_stacks: Vec<Chips>,
    /// `final_stack - starting_stack` in mbb.
    pub net: Vec<i64>,
}

/// Event-sourced state of one no-limit hold'em hand. Serialize-only: a state can only be
/// reached by replaying validated events, never by deserializing arbitrary data.
#[derive(Debug, Clone, Serialize)]
pub struct GameState {
    config: GameConfig,
    players: Vec<PlayerState>,
    hole_cards: Vec<Option<HoleCards>>,
    board: Board,
    street: Street,
    phase: Phase,
    current_bet: Chips,
    /// Size of the last full bet or raise on this street (minimum raise increment).
    last_raise_size: Chips,
    history: Vec<ActionRecord>,
    saw_flop: bool,
}

impl GameState {
    /// Starts a hand where every player has the configured effective stack.
    pub fn new(config: GameConfig) -> Result<GameState> {
        let stacks = vec![config.stack.effective_stack; config.players as usize];
        Self::with_stacks(config, &stacks)
    }

    /// Starts a hand with explicit per-seat stacks (seat order = preflop action order).
    pub fn with_stacks(config: GameConfig, stacks: &[Chips]) -> Result<GameState> {
        config.validate()?;
        let positions = Position::for_players(config.players as usize)?;
        if stacks.len() != positions.len() {
            return Err(PokerError::InvalidConfig(format!(
                "expected {} stacks, got {}",
                positions.len(),
                stacks.len()
            )));
        }
        let bb = GameConfig::BIG_BLIND;
        let n = positions.len();
        let mut players = Vec::with_capacity(n);
        for (i, (&position, &stack)) in positions.iter().zip(stacks).enumerate() {
            let blind = match i {
                _ if i == n - 1 => bb,
                _ if i == n - 2 => config.small_blind,
                _ => Chips::ZERO,
            };
            if stack > crate::stack::MAX_EFFECTIVE_STACK {
                return Err(PokerError::InvalidConfig(format!(
                    "{position} stack {stack} exceeds the maximum of {}",
                    crate::stack::MAX_EFFECTIVE_STACK
                )));
            }
            if stack <= config.ante + blind {
                return Err(PokerError::InvalidConfig(format!(
                    "{position} stack {stack} cannot cover ante and blind with chips behind"
                )));
            }
            players.push(PlayerState {
                position,
                starting_stack: stack,
                stack: stack - config.ante - blind,
                street_commit: blind,
                total_commit: config.ante + blind,
                folded: false,
                all_in: false,
                acted: false,
                reraise_threshold: None,
            });
        }
        let mut s = GameState {
            hole_cards: vec![None; n],
            players,
            board: Board::empty(),
            street: Street::Preflop,
            phase: Phase::Betting { to_act: 0 },
            current_bet: bb,
            last_raise_size: bb,
            history: Vec::new(),
            saw_flop: false,
            config,
        };
        s.phase = Phase::Betting { to_act: s.next_to_act(n - 1).expect("preflop always has an actor") };
        Ok(s)
    }

    // ---------- accessors ----------

    pub fn config(&self) -> &GameConfig {
        &self.config
    }
    pub fn players(&self) -> &[PlayerState] {
        &self.players
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn street(&self) -> Street {
        self.street
    }
    pub fn board(&self) -> &Board {
        &self.board
    }
    pub fn history(&self) -> &[ActionRecord] {
        &self.history
    }
    pub fn current_bet(&self) -> Chips {
        self.current_bet
    }
    pub fn min_raise_increment(&self) -> Chips {
        self.last_raise_size
    }
    pub fn saw_flop(&self) -> bool {
        self.saw_flop
    }
    pub fn hole_cards(&self, seat: usize) -> Option<HoleCards> {
        self.hole_cards.get(seat).copied().flatten()
    }
    /// Total chips committed by all players (the pot, including the current street).
    pub fn pot_total(&self) -> Chips {
        self.players.iter().map(|p| p.total_commit).sum()
    }
    pub fn to_act(&self) -> Option<usize> {
        match self.phase {
            Phase::Betting { to_act } => Some(to_act),
            _ => None,
        }
    }
    /// Effective stack (chips behind) of `seat` against the live opponents.
    pub fn effective_stack(&self, seat: usize) -> Option<Chips> {
        let opp: Vec<Chips> =
            self.players.iter().enumerate().filter(|(i, p)| *i != seat && !p.folded).map(|(_, p)| p.stack).collect();
        effective_stack(self.players.get(seat)?.stack, &opp)
    }
    /// Current main/side pots.
    pub fn pots(&self) -> Vec<Pot> {
        let contrib: Vec<Chips> = self.players.iter().map(|p| p.total_commit).collect();
        let folded: Vec<bool> = self.players.iter().map(|p| p.folded).collect();
        compute_pots(&contrib, &folded)
    }

    fn live_count(&self) -> usize {
        self.players.iter().filter(|p| !p.folded).count()
    }
    fn can_act(p: &PlayerState) -> bool {
        !p.folded && !p.all_in
    }
    fn order(&self) -> Vec<usize> {
        let n = self.players.len();
        if self.street == Street::Preflop { (0..n).collect() } else { Position::postflop_order(n) }
    }
    fn needs_to_act(&self, i: usize) -> bool {
        let p = &self.players[i];
        if !Self::can_act(p) {
            return false;
        }
        if p.street_commit < self.current_bet {
            return true;
        }
        if p.acted {
            return false;
        }
        // Unacted and not facing a bet: act only if someone else could still respond.
        self.players.iter().enumerate().any(|(j, q)| j != i && Self::can_act(q))
    }
    /// The next seat after `after` (in this street's order) that needs to act.
    fn next_to_act(&self, after: usize) -> Option<usize> {
        let order = self.order();
        let start = order.iter().position(|&s| s == after).map_or(0, |k| k + 1);
        (0..order.len()).map(|k| order[(start + k) % order.len()]).find(|&s| self.needs_to_act(s))
    }

    // ---------- cards ----------

    fn used_cards(&self, except_seat: Option<usize>) -> CardSet {
        let mut used = self.board.card_set();
        for (i, h) in self.hole_cards.iter().enumerate() {
            if Some(i) != except_seat {
                if let Some(h) = h {
                    used = used.union(h.card_set());
                }
            }
        }
        used
    }

    /// Records a player's hole cards; they must not collide with the board or other players.
    pub fn set_hole_cards(&mut self, seat: usize, cards: HoleCards) -> Result<()> {
        if seat >= self.players.len() {
            return Err(PokerError::InvalidConfig(format!("no seat {seat}")));
        }
        if let Some(c) = cards.card_set().iter().find(|c| self.used_cards(Some(seat)).contains(*c)) {
            return Err(PokerError::DuplicateCard(c.to_string()));
        }
        self.hole_cards[seat] = Some(cards);
        Ok(())
    }

    /// Deals the community cards for the street the hand is waiting for.
    pub fn deal_board(&mut self, cards: &[Card]) -> Result<()> {
        let Phase::AwaitingBoard { street } = self.phase else {
            return Err(PokerError::InvalidTransition(format!("cannot deal the board in phase {:?}", self.phase)));
        };
        let used = self.used_cards(None);
        if let Some(c) = cards.iter().find(|c| used.contains(**c)) {
            return Err(PokerError::DuplicateCard(c.to_string()));
        }
        let dealt = self.board.deal_next(cards)?;
        debug_assert_eq!(dealt, street);
        self.street = street;
        self.saw_flop = true;
        self.start_street();
        Ok(())
    }

    fn start_street(&mut self) {
        for p in &mut self.players {
            p.street_commit = Chips::ZERO;
            p.acted = false;
            p.reraise_threshold = None;
        }
        self.current_bet = Chips::ZERO;
        self.last_raise_size = GameConfig::BIG_BLIND;
        let first = Position::postflop_order(self.players.len()).into_iter().find(|&s| self.needs_to_act(s));
        self.phase = match first {
            Some(s) => Phase::Betting { to_act: s },
            None => self.after_street_phase(),
        };
    }

    fn after_street_phase(&self) -> Phase {
        match self.street.next() {
            Some(next) => Phase::AwaitingBoard { street: next },
            None => Phase::Showdown,
        }
    }

    // ---------- actions ----------

    pub fn legal_actions(&self) -> Result<LegalActions> {
        let seat = self
            .to_act()
            .ok_or_else(|| PokerError::InvalidTransition(format!("no player to act in phase {:?}", self.phase)))?;
        let p = &self.players[seat];
        let to_call = self.current_bet.checked_sub(p.street_commit).unwrap_or(Chips::ZERO);
        let max_to = p.street_commit + p.stack;
        let opponent_can_respond = self.players.iter().enumerate().any(|(j, q)| j != seat && Self::can_act(q));
        let mut la = LegalActions { seat, fold: false, check: false, call: None, bet: None, raise: None };
        if to_call.is_zero() {
            la.check = true;
            if self.current_bet.is_zero() && opponent_can_respond {
                let min_to = GameConfig::BIG_BLIND.min(max_to);
                la.bet = Some((min_to, max_to));
            } else if opponent_can_respond && self.reraise_allowed(p) && max_to > self.current_bet {
                // e.g. the big blind's option preflop
                let min_to = (self.current_bet + self.last_raise_size).min(max_to);
                la.raise = Some((min_to, max_to));
            }
        } else {
            la.fold = true;
            la.call = Some(to_call.min(p.stack));
            if p.stack > to_call && opponent_can_respond && self.reraise_allowed(p) {
                let min_to = (self.current_bet + self.last_raise_size).min(max_to);
                la.raise = Some((min_to, max_to));
            }
        }
        Ok(la)
    }

    fn reraise_allowed(&self, p: &PlayerState) -> bool {
        p.reraise_threshold.is_none_or(|t| self.current_bet >= t)
    }

    /// Applies the action of the player to act.
    pub fn apply(&mut self, action: Action) -> Result<()> {
        let la = self.legal_actions()?;
        let seat = la.seat;
        let illegal = |why: String| Err(PokerError::IllegalAction(why));
        let (added, full_raise) = match action {
            Action::Fold => {
                if !la.fold {
                    return illegal("cannot fold when checking is free".into());
                }
                self.players[seat].folded = true;
                (Chips::ZERO, false)
            }
            Action::Check => {
                if !la.check {
                    return illegal(format!("cannot check facing {}", self.current_bet));
                }
                (Chips::ZERO, false)
            }
            Action::Call => match la.call {
                Some(amount) => (amount, false),
                None => return illegal("nothing to call".into()),
            },
            Action::Bet { to } => {
                let Some((min, max)) = la.bet else {
                    return illegal("betting is not available (there is already a bet)".into());
                };
                if to > max || (to < min && to != max) || to.is_zero() {
                    return illegal(format!("bet to {to} must be between {min} and {max} (or all-in)"));
                }
                let full = to >= GameConfig::BIG_BLIND;
                if full {
                    self.last_raise_size = to;
                }
                self.current_bet = to;
                // An opening bet re-opens action for everybody.
                for (j, q) in self.players.iter_mut().enumerate() {
                    if j != seat {
                        q.reraise_threshold = None;
                    }
                }
                (to - self.players[seat].street_commit, full)
            }
            Action::Raise { to } => {
                let Some((min, max)) = la.raise else {
                    return illegal("raising is not available".into());
                };
                if to > max || (to < min && to != max) {
                    return illegal(format!("raise to {to} must be between {min} and {max} (or all-in)"));
                }
                let increment = to - self.current_bet;
                let full = increment >= self.last_raise_size;
                if full {
                    self.last_raise_size = increment;
                }
                self.current_bet = to;
                (to - self.players[seat].street_commit, full)
            }
        };
        let threshold = self.current_bet + self.last_raise_size;
        let p = &mut self.players[seat];
        p.stack -= added;
        p.street_commit += added;
        p.total_commit += added;
        p.acted = true;
        p.all_in = p.stack.is_zero();
        p.reraise_threshold = Some(threshold);
        self.history.push(ActionRecord {
            seat,
            position: p.position,
            street: self.street,
            action,
            added,
            street_commit: p.street_commit,
            all_in: p.all_in,
            full_raise,
        });
        self.advance(seat);
        Ok(())
    }

    fn advance(&mut self, last: usize) {
        if self.live_count() == 1 {
            self.return_uncalled();
            let winner = self.players.iter().position(|p| !p.folded).expect("one live player");
            self.phase = Phase::Uncontested { winner };
            return;
        }
        match self.next_to_act(last) {
            Some(s) => self.phase = Phase::Betting { to_act: s },
            None => {
                self.return_uncalled();
                self.phase = self.after_street_phase();
            }
        }
    }

    /// Returns the part of the largest street commitment that nobody matched.
    fn return_uncalled(&mut self) {
        let mut idx: Vec<usize> = (0..self.players.len()).collect();
        idx.sort_by(|&a, &b| self.players[b].street_commit.cmp(&self.players[a].street_commit));
        let (top, second) = (idx[0], idx[1]);
        let excess = self.players[top].street_commit - self.players[second].street_commit;
        if !excess.is_zero() {
            let p = &mut self.players[top];
            p.street_commit -= excess;
            p.total_commit -= excess;
            p.stack += excess;
            p.all_in = p.stack.is_zero();
            self.current_bet = p.street_commit;
        }
    }

    // ---------- settlement ----------

    /// Settles a hand won without showdown.
    pub fn settle_uncontested(&self) -> Result<Settlement> {
        let Phase::Uncontested { .. } = self.phase else {
            return Err(PokerError::InvalidTransition("hand was not won uncontested".into()));
        };
        let values: Vec<Option<u8>> = self.players.iter().map(|p| (!p.folded).then_some(0)).collect();
        self.settle_inner(&values)
    }

    /// Settles a showdown. `values[seat]` is the hand strength of every live player (higher
    /// wins, equal values split); folded seats may be `None`.
    pub fn settle_showdown<V: Ord + Copy>(&self, values: &[Option<V>]) -> Result<Settlement> {
        if self.phase != Phase::Showdown {
            return Err(PokerError::InvalidTransition(format!("cannot settle a showdown in phase {:?}", self.phase)));
        }
        if values.len() != self.players.len() {
            return Err(PokerError::InvalidTransition("one value per seat is required".into()));
        }
        if let Some(i) = (0..self.players.len()).find(|&i| !self.players[i].folded && values[i].is_none()) {
            return Err(PokerError::InvalidTransition(format!("missing showdown value for live seat {i}")));
        }
        self.settle_inner(values)
    }

    fn settle_inner<V: Ord + Copy>(&self, values: &[Option<V>]) -> Result<Settlement> {
        let pots = self.pots();
        let total = self.pot_total();
        let rake = self.config.rake.compute(total, self.saw_flop, self.config.stake.as_ref())?;
        let amounts: Vec<Chips> = pots.iter().map(|p| p.amount).collect();
        let rake_shares = self.config.rake.allocate(rake.rake, &amounts)?;
        let n = self.players.len();
        // Odd chips go to the winner closest to the left of the button (postflop order).
        let odd_order = Position::postflop_order(n);
        let mut payouts = vec![Chips::ZERO; n];
        let mut results = Vec::with_capacity(pots.len());
        for (pot, pot_rake) in pots.into_iter().zip(rake_shares) {
            let best = pot.eligible.iter().filter_map(|&i| values[i]).max();
            let mut winners: Vec<usize> =
                pot.eligible.iter().copied().filter(|&i| values[i].is_some() && values[i] == best).collect();
            winners.sort_by_key(|w| odd_order.iter().position(|s| s == w));
            let net = pot.amount - pot_rake;
            let k = winners.len() as u64;
            let (each, rem) = (net.mbb() / k, net.mbb() % k);
            let shares: Vec<Chips> = (0..winners.len()).map(|j| Chips(each + u64::from((j as u64) < rem))).collect();
            for (w, s) in winners.iter().zip(&shares) {
                payouts[*w] += *s;
            }
            results.push(PotResult { amount: pot.amount, rake: pot_rake, eligible: pot.eligible, winners, shares });
        }
        let final_stacks: Vec<Chips> = self.players.iter().zip(&payouts).map(|(p, w)| p.stack + *w).collect();
        let net = self
            .players
            .iter()
            .zip(&final_stacks)
            .map(|(p, f)| f.mbb() as i64 - p.starting_stack.mbb() as i64)
            .collect();
        Ok(Settlement { pots: results, rake, payouts, final_stacks, net })
    }
}
