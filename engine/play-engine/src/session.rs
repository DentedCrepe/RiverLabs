//! A play session: one human ("hero", always player 0) against bots, hand after hand.

use crate::bot::{self, BotDecision, DecisionSource, Situation};
use crate::config::{Difficulty, Format, Level, PlayError, Result, TableConfig};
use crate::model::{Model, ModelCache, ModelShape};
use crate::record::{AllInSnapshot, HandRecord, HeroDecision, ModelCheck, RecordAction, RecordSeat};
use crate::strength::card_set;
use poker_core::game::{Action, GameState, Phase};
use poker_core::{Card, Chips, GameConfig, HoleCards, Street};
use rand::seq::SliceRandom;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use range_engine::{COMBO_COUNT, Combo, Range};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct Player {
    pub name: String,
    /// `None` for you.
    pub difficulty: Option<Difficulty>,
    /// Chips (mbb at cash tables, tournament chips in a sit & go).
    pub chips: u64,
    pub busted: bool,
    /// Sit & go finishing place (1 = winner).
    pub finish: Option<u8>,
    pub rebuys: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    AwaitingHero,
    HandOver,
    SessionOver,
}

/// State of the hand being played.
#[derive(Debug)]
struct Hand {
    state: GameState,
    /// Game seat (preflop action order) -> player index.
    seat_player: Vec<usize>,
    deck: Vec<Card>,
    hole: Vec<HoleCards>,
    rng: ChaCha8Rng,
    /// Tracked range per game seat (bots: their exact policy range; hero: unused).
    ranges: Vec<Vec<f64>>,
    model: Option<Arc<Model>>,
    /// Path of model action indices; `None` once the hand has left the model tree.
    model_path: Option<Vec<usize>>,
    record: HandRecord,
    decisions: u64,
    /// When you are to act: your equity against the live bots' tracked ranges, and the equity
    /// the price requires (if facing a bet).
    hero_odds: Option<HeroOdds>,
    /// Your range at the flop in the bots' preflop model (for "solve this spot").
    hero_range: Option<String>,
    /// The tracked ranges when the current street was dealt: a solve of a street starts there,
    /// before the bot's own actions on that street narrow its range.
    street_ranges: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct HeroOdds {
    pub equity: Option<f64>,
    pub required: Option<f64>,
}

#[derive(Debug)]
pub struct Session {
    cfg: TableConfig,
    pub players: Vec<Player>,
    button: usize,
    hand_no: u64,
    hand: Option<Hand>,
    records: Vec<HandRecord>,
    status: SessionStatus,
    /// Practice mode: show the bots' cards during hands (hands are flagged in the report).
    reveal: bool,
    /// Model notes for the view (e.g. which preflop model the bots use).
    notes: Vec<String>,
}

fn starting_chips(cfg: &TableConfig) -> u64 {
    match &cfg.format {
        Format::Cash { game, .. } | Format::FastFold { game } => game.stack.effective_stack.mbb(),
        Format::SitAndGo { starting_chips, .. } => *starting_chips,
    }
}

impl Session {
    pub fn new(cfg: TableConfig) -> Result<Session> {
        cfg.validate()?;
        let chips = starting_chips(&cfg);
        let mut players =
            vec![Player { name: "You".into(), difficulty: None, chips, busted: false, finish: None, rebuys: 0 }];
        for (i, d) in cfg.opponents.iter().enumerate() {
            players.push(Player {
                name: format!("Bot {} ({})", i + 1, difficulty_name(*d)),
                difficulty: Some(*d),
                chips,
                busted: false,
                finish: None,
                rebuys: 0,
            });
        }
        let button = (cfg.seed % players.len() as u64) as usize;
        Ok(Session {
            cfg,
            players,
            button,
            hand_no: 0,
            hand: None,
            records: Vec::new(),
            status: SessionStatus::HandOver,
            notes: Vec::new(),
            reveal: false,
        })
    }

    pub fn config(&self) -> &TableConfig {
        &self.cfg
    }
    pub fn status(&self) -> SessionStatus {
        self.status
    }
    pub fn records(&self) -> &[HandRecord] {
        &self.records
    }
    pub fn hands_played(&self) -> u64 {
        self.records.len() as u64
    }

    /// The blind level for the next hand: (small blind, big blind, ante) in chip units, and
    /// whether amounts are tournament chips.
    fn level(&self) -> (Chips, Chips, Chips, Option<usize>) {
        match &self.cfg.format {
            Format::Cash { game, .. } | Format::FastFold { game } => {
                (game.small_blind, GameConfig::BIG_BLIND, game.ante, None)
            }
            Format::SitAndGo { levels, hands_per_level, .. } => {
                let i = ((self.hand_no / u64::from(*hands_per_level)) as usize).min(levels.len() - 1);
                let Level { small_blind, big_blind, ante } = levels[i];
                (Chips(small_blind), Chips(big_blind), Chips(ante), Some(i))
            }
        }
    }

    /// Solves (or loads) the bots' preflop model for the starting table so the first hand
    /// starts quickly. Optional: `start_hand` does the same on demand.
    pub fn prepare(&self, models: &ModelCache) -> Result<()> {
        let (_, bb, _, _) = self.level();
        let n = self.players.iter().filter(|p| !p.busted).count();
        models.get(&self.model_shape(n, starting_chips(&self.cfg) as f64 / bb.mbb() as f64)).map(|_| ())
    }

    /// The model shape for `players` at `effective_bb`, with this level's blinds converted to bb
    /// (rounded to 0.001bb) and the table's rake.
    fn model_shape(&self, players: usize, effective_bb: f64) -> ModelShape {
        let (sb, bb, ante, _) = self.level();
        let rake = match &self.cfg.format {
            Format::Cash { game, .. } | Format::FastFold { game } => game.rake,
            Format::SitAndGo { .. } => poker_core::rake::RakeConfig::none(),
        };
        ModelShape {
            players: players as u8,
            stack_bb: ModelShape::bucket(effective_bb),
            small_blind_mbb: (sb.mbb() * 1000 + bb.mbb() / 2) / bb.mbb(),
            ante_mbb: (ante.mbb() * 1000 + bb.mbb() / 2) / bb.mbb(),
            rake,
        }
    }

    /// Ends the session early (you can always leave).
    pub fn leave(&mut self) {
        self.status = SessionStatus::SessionOver;
        self.hand = None;
    }

    /// Deals the next hand and plays bots until it is your turn or the hand ends.
    pub fn start_hand(&mut self, models: &ModelCache) -> Result<()> {
        if self.status == SessionStatus::SessionOver {
            return Err(PlayError::State("the session is over".into()));
        }
        if self.status == SessionStatus::AwaitingHero {
            return Err(PlayError::State("finish the current hand first".into()));
        }
        let mut rng = ChaCha8Rng::seed_from_u64(self.cfg.seed ^ (self.hand_no + 1).wrapping_mul(0xA24B_AED4_963E_E407));
        let start = starting_chips(&self.cfg);
        if let Format::FastFold { .. } = self.cfg.format {
            // A fresh table: new opponents (shuffled difficulties, starting stacks), random seat.
            let mut diffs = self.cfg.opponents.clone();
            diffs.shuffle(&mut rng);
            for (i, d) in diffs.into_iter().enumerate() {
                let p = &mut self.players[i + 1];
                p.difficulty = Some(d);
                p.name = format!("Player {} ({})", (b'A' + i as u8) as char, difficulty_name(d));
                p.chips = start;
            }
            if self.players[0].chips < start {
                self.players[0].rebuys += 1;
                self.players[0].chips = start;
            }
            self.button = rng.random_range(0..self.players.len());
        }
        let active: Vec<usize> = (0..self.players.len()).filter(|&i| !self.players[i].busted).collect();
        let n = active.len();
        if n < 2 {
            self.status = SessionStatus::SessionOver;
            return Err(PlayError::State("not enough players left".into()));
        }
        // Seat mapping: game seat k (preflop order UTG..BTN, SB, BB) -> player.
        let bpos = active.iter().position(|&p| p == self.button).unwrap_or(0);
        let seat_player: Vec<usize> =
            (0..n).map(|k| if n == 2 { active[(bpos + k) % 2] } else { active[(bpos + 3 + k) % n] }).collect();
        let stacks: Vec<Chips> = seat_player.iter().map(|&p| Chips(self.players[p].chips)).collect();
        let (sb, bb, ante, level) = self.level();
        let state = match &self.cfg.format {
            Format::Cash { game, .. } | Format::FastFold { game } => {
                let cfg = GameConfig { players: n as u8, ..game.clone() };
                GameState::with_stacks(cfg, &stacks)?
            }
            Format::SitAndGo { .. } => GameState::tournament(sb, bb, ante, &stacks)?,
        };
        let mut deck: Vec<Card> = Card::all().collect();
        deck.shuffle(&mut rng);
        let mut state = state;
        let mut hole = Vec::with_capacity(n);
        for k in 0..n {
            let h = HoleCards::new(deck.pop().expect("deck"), deck.pop().expect("deck"))?;
            state.set_hole_cards(k, h)?;
            hole.push(h);
        }

        // The bots' preflop model for this table shape (effective stack = second-largest stack).
        let mut sorted: Vec<u64> = stacks.iter().map(|c| c.mbb()).collect();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        let effective_bb = sorted[1].min(sorted[0]) as f64 / bb.mbb() as f64;
        let shape = self.model_shape(n, effective_bb);
        let (model, model_note, used_bb) = match models.get_or_nearest(&shape) {
            Ok((m, used)) => (Some(m), None, used),
            Err(e) => (
                None,
                Some(format!("The preflop model was unavailable ({e}); bots used the fallback rules.")),
                shape.stack_bb,
            ),
        };
        // In a tournament the next bust needs a model for one player fewer: prepare it now.
        if level.is_some() && n > 2 {
            models.prepare_in_background(&self.model_shape(n - 1, effective_bb));
        }
        self.notes = vec![match &model_note {
            Some(n) => n.clone(),
            None if used_bb != shape.stack_bb => format!(
                "Bots' preflop: GTOpen's preflop model for {n} players at {used_bb}bb, the nearest stack depth \
                 prepared so far (approximate model). The {}bb model is being prepared and is used once it is \
                 ready. Postflop: rule-based bots, not a solver.",
                shape.stack_bb
            ),
            None => format!(
                "Bots' preflop: GTOpen's preflop model for {n} players at {}bb (approximate model; your stack and bet \
                 sizes are mapped to the nearest model spot). Postflop: rule-based bots, not a solver.",
                shape.stack_bb
            ),
        }];

        let record = HandRecord {
            hand_no: self.hand_no + 1,
            level: level.map(|i| i + 1),
            tournament: level.is_some(),
            small_blind: sb.mbb(),
            big_blind: bb.mbb(),
            ante: ante.mbb(),
            seats: seat_player
                .iter()
                .enumerate()
                .map(|(k, &p)| RecordSeat {
                    name: self.players[p].name.clone(),
                    difficulty: self.players[p].difficulty,
                    position: state.players()[k].position.short_name().into(),
                    starting_stack: stacks[k].mbb(),
                    hole_cards: hole[k].cards().iter().map(|c| c.to_string()).collect(),
                    shown: false,
                    is_hero: p == 0,
                })
                .collect(),
            board: vec![],
            actions: vec![],
            net: vec![0; n],
            showdown: false,
            hero_decisions: vec![],
            all_in: None,
            hero_all_in_ev: None,
            model_stack_bb: model.as_ref().map(|_| shape.stack_bb),
            notes: model_note.into_iter().collect(),
            revealed: self.reveal,
            summary: vec![],
        };
        self.hand = Some(Hand {
            state,
            seat_player,
            deck,
            hole,
            rng,
            ranges: vec![vec![1.0; COMBO_COUNT]; n],
            model,
            model_path: Some(vec![]),
            record,
            decisions: 0,
            hero_odds: None,
            hero_range: None,
            street_ranges: vec![],
        });
        self.hand_no += 1;
        self.run()
    }

    /// Your action. Amounts are "to" amounts in chip units (mbb at cash tables).
    pub fn act(&mut self, action: Action) -> Result<()> {
        if self.status != SessionStatus::AwaitingHero {
            return Err(PlayError::State("it is not your turn".into()));
        }
        let hand = self.hand.as_mut().expect("a hand is in progress");
        let Phase::Betting { to_act } = hand.state.phase() else {
            return Err(PlayError::State("no betting in progress".into()));
        };
        let mut decision = hero_decision(hand, to_act, action);
        let before = hand.state.clone();
        hand.state.apply(action).map_err(|e| PlayError::IllegalAction(e.to_string()))?;
        let tournament = hand.record.tournament;
        let amount = |x: u64| if tournament { x.to_string() } else { Chips(x).to_string() };
        let judgement = crate::judge::judge(&decision, &hand.record.seats[to_act].position, amount);
        decision.feedback = Some(crate::judge::feedback(&decision, &judgement));
        hand.record.hero_decisions.push(decision);
        if self.reveal {
            hand.record.revealed = true;
        }
        let model_action = translate(hand, &before, to_act, action);
        after_action(hand, &before, to_act, action, model_action);
        self.status = SessionStatus::HandOver; // until `run` decides otherwise
        self.run()
    }

    fn run(&mut self) -> Result<()> {
        loop {
            let hand = self.hand.as_mut().expect("a hand is in progress");
            match hand.state.phase() {
                Phase::Betting { to_act } => {
                    if hand.seat_player[to_act] == 0 {
                        let state = &hand.state;
                        let to_call = state.legal_actions().ok().and_then(|l| l.call).map_or(0, |c| c.mbb());
                        let pot = state.pot_total().mbb();
                        let opponents: Vec<usize> =
                            (0..state.players().len()).filter(|&k| k != to_act && !state.players()[k].folded).collect();
                        let board = state.board().cards().to_vec();
                        hand.hero_odds = Some(HeroOdds {
                            equity: equity_vs_ranges(hand, to_act, &opponents, &board),
                            required: (to_call > 0).then(|| to_call as f64 / (pot + to_call) as f64),
                        });
                        self.status = SessionStatus::AwaitingHero;
                        return Ok(());
                    }
                    let difficulty = self.players[hand.seat_player[to_act]].difficulty.unwrap_or(Difficulty::Hard);
                    let d = bot_decide(hand, to_act, difficulty)?;
                    let before = hand.state.clone();
                    if let Err(e) = hand.state.apply(d.action) {
                        return Err(PlayError::Engine(format!("bot chose an illegal action {:?}: {e}", d.action)));
                    }
                    for (w, p) in hand.ranges[to_act].iter_mut().zip(&d.combo_probability) {
                        *w *= p;
                    }
                    let model_action = if d.source == DecisionSource::PreflopModel { d.model_action } else { None };
                    after_action(hand, &before, to_act, d.action, model_action);
                }
                Phase::AwaitingBoard { street } => {
                    if hand.record.all_in.is_none() && betting_closed(&hand.state) {
                        hand.record.all_in = Some(AllInSnapshot {
                            street: street_name(hand.state.street()).into(),
                            board: hand.state.board().cards().iter().map(|c| c.to_string()).collect(),
                        });
                    }
                    if street == Street::Flop {
                        hand.hero_range = hero_model_range(hand);
                    }
                    let k = if street == Street::Flop { 3 } else { 1 };
                    let cards: Vec<Card> = (0..k).map(|_| hand.deck.pop().expect("deck")).collect();
                    hand.state.deal_board(&cards)?;
                    hand.street_ranges = hand.ranges.clone();
                    hand.model_path = None; // the preflop model ends at the flop
                }
                Phase::Showdown => {
                    let board = hand.state.board().cards().to_vec();
                    let values: Vec<Option<hand_evaluator::HandStrength>> = (0..hand.hole.len())
                        .map(|k| {
                            if hand.state.players()[k].folded {
                                None
                            } else {
                                let mut cards = board.clone();
                                cards.extend(hand.hole[k].cards());
                                hand_evaluator::evaluate(&cards).ok()
                            }
                        })
                        .collect();
                    let settlement = hand.state.settle_showdown(&values)?;
                    hand.record.showdown = true;
                    for (k, s) in hand.record.seats.iter_mut().enumerate() {
                        s.shown = values[k].is_some();
                    }
                    return self.finish_hand(settlement, Some(values));
                }
                Phase::Uncontested { .. } => {
                    let settlement = hand.state.settle_uncontested()?;
                    return self.finish_hand(settlement, None);
                }
            }
        }
    }

    fn finish_hand(
        &mut self,
        settlement: poker_core::game::Settlement,
        values: Option<Vec<Option<hand_evaluator::HandStrength>>>,
    ) -> Result<()> {
        let mut hand = self.hand.take().expect("hand");
        hand.record.board = hand.state.board().cards().iter().map(|c| c.to_string()).collect();
        hand.record.net = settlement.net.clone();
        hand.record.summary = summary(&hand.record, &settlement, values.as_deref());
        if let Some(snap) = &hand.record.all_in {
            hand.record.hero_all_in_ev = crate::record::all_in_ev(&hand.record, snap, &settlement);
        }
        for (k, &p) in hand.seat_player.iter().enumerate() {
            self.players[p].chips = settlement.final_stacks[k].mbb();
        }
        let start = starting_chips(&self.cfg);
        let mut over = false;
        match &self.cfg.format {
            Format::Cash { auto_rebuy, .. } => {
                for (i, p) in self.players.iter_mut().enumerate() {
                    if p.chips == 0 {
                        if i == 0 && !auto_rebuy {
                            over = true;
                        } else {
                            p.chips = start;
                            p.rebuys += 1;
                        }
                    }
                }
            }
            Format::FastFold { .. } => {
                if self.players[0].chips == 0 {
                    self.players[0].chips = start;
                    self.players[0].rebuys += 1;
                }
            }
            Format::SitAndGo { .. } => {
                let left = self.players.iter().filter(|p| !p.busted).count();
                let newly: Vec<usize> = (0..self.players.len())
                    .filter(|&i| !self.players[i].busted && self.players[i].chips == 0)
                    .collect();
                // Players busting in the same hand share the worse places, bigger starting stack first.
                let mut order = newly.clone();
                order.sort_by_key(|&i| {
                    std::cmp::Reverse(
                        hand.seat_player
                            .iter()
                            .position(|&p| p == i)
                            .map_or(0, |k| hand.record.seats[k].starting_stack),
                    )
                });
                for (j, &i) in order.iter().enumerate() {
                    self.players[i].busted = true;
                    self.players[i].finish = Some((left - j) as u8);
                }
                let remaining: Vec<usize> = (0..self.players.len()).filter(|&i| !self.players[i].busted).collect();
                if remaining.len() == 1 {
                    self.players[remaining[0]].finish = Some(1);
                    over = true;
                }
                if self.players[0].busted {
                    over = true;
                }
            }
        }
        self.records.push(hand.record);
        // Move the button to the next player still in.
        let n = self.players.len();
        let mut b = (self.button + 1) % n;
        while self.players[b].busted && b != self.button {
            b = (b + 1) % n;
        }
        self.button = b;
        self.status = if over { SessionStatus::SessionOver } else { SessionStatus::HandOver };
        Ok(())
    }

    pub fn view(&self) -> crate::view::TableView {
        crate::view::build(
            self,
            self.hand.as_ref().map(|h| crate::view::HandRef {
                state: &h.state,
                seat_player: &h.seat_player,
                hole: &h.hole,
                record: &h.record,
                hero_odds: if self.status == SessionStatus::AwaitingHero { h.hero_odds } else { None },
            }),
            &self.notes,
        )
    }

    /// Practice mode: show the bots' cards during hands. Hands played with it on are flagged.
    pub fn set_reveal(&mut self, on: bool) {
        self.reveal = on;
        if on {
            if let Some(h) = self.hand.as_mut() {
                h.record.revealed = true;
            }
        }
    }
    pub fn reveal(&self) -> bool {
        self.reveal
    }

    pub fn button(&self) -> usize {
        self.button
    }
    pub fn level_info(&self) -> Option<(usize, Level)> {
        match &self.cfg.format {
            Format::SitAndGo { levels, hands_per_level, .. } => {
                let i = ((self.hand_no.saturating_sub(1) / u64::from(*hands_per_level)) as usize).min(levels.len() - 1);
                Some((i + 1, levels[i]))
            }
            _ => None,
        }
    }
}

/// "Bot 2 wins 12.5bb with Two Pair, Kings and Sevens" for each pot.
fn summary(
    record: &HandRecord,
    settlement: &poker_core::game::Settlement,
    values: Option<&[Option<hand_evaluator::HandStrength>]>,
) -> Vec<String> {
    let amount = |x: u64| if record.tournament { format!("{x} chips") } else { Chips(x).to_string() };
    let side = settlement.pots.len() > 1;
    settlement
        .pots
        .iter()
        .enumerate()
        .flat_map(|(i, pot)| {
            let label = match (side, i) {
                (false, _) => String::new(),
                (true, 0) => " the main pot".to_string(),
                (true, n) => format!(" side pot {n}"),
            };
            pot.winners.iter().zip(&pot.shares).map(move |(&w, share)| {
                let how = match values.and_then(|v| v[w]) {
                    Some(h) => format!(" with {}", h.describe()),
                    None => " (everyone else folded)".to_string(),
                };
                let (who, verb) =
                    if record.seats[w].is_hero { ("You", "win") } else { (record.seats[w].name.as_str(), "wins") };
                format!("{who} {verb} {}{label}{how}", amount(share.mbb()))
            })
        })
        .collect()
}

pub fn difficulty_name(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Easy => "easy",
        Difficulty::Medium => "medium",
        Difficulty::Hard => "hard",
    }
}

pub fn street_name(s: Street) -> &'static str {
    match s {
        Street::Preflop => "preflop",
        Street::Flop => "flop",
        Street::Turn => "turn",
        Street::River => "river",
    }
}

/// No further betting is possible: at most one live player still has chips behind.
fn betting_closed(state: &GameState) -> bool {
    let live: Vec<_> = state.players().iter().filter(|p| !p.folded).collect();
    live.len() >= 2 && live.iter().filter(|p| !p.all_in).count() <= 1
}

fn bot_decide(hand: &mut Hand, seat: usize, difficulty: Difficulty) -> Result<BotDecision> {
    hand.decisions += 1;
    let legal = hand.state.legal_actions()?;
    let p = &hand.state.players()[seat];
    let board = hand.state.board().cards().to_vec();
    let situation = Situation {
        hole: hand.hole[seat],
        board: &board,
        legal,
        difficulty,
        pot: hand.state.pot_total(),
        stack_to: p.stack + p.street_commit,
        current_bet: hand.state.current_bet(),
        big_blind: hand.state.big_blind(),
        live_players: hand.state.players().iter().filter(|q| !q.folded).count(),
        range: &hand.ranges[seat],
        seed: hand.record.hand_no ^ hand.decisions.wrapping_mul(0x9E37_79B9),
    };
    if board.is_empty() {
        if let (Some(model), Some(path)) = (&hand.model, &hand.model_path) {
            if let Some(node) = model.node(path) {
                if node.kind == "action" && node.actor.as_deref() == Some(p.position.short_name()) {
                    if let Some(d) = bot::preflop_model(&node, &situation, &mut hand.rng) {
                        return Ok(d);
                    }
                }
            }
        }
    }
    Ok(bot::rules(&situation, situation.seed))
}

/// Maps a real action to the model action at the current node (`None` = left the tree).
fn translate(hand: &Hand, before: &GameState, seat: usize, action: Action) -> Option<usize> {
    if !before.board().cards().is_empty() {
        return None;
    }
    let node = hand.model.as_ref()?.node(hand.model_path.as_ref()?)?;
    if node.kind != "action" || node.actor.as_deref() != Some(before.players()[seat].position.short_name()) {
        return None;
    }
    let p = &before.players()[seat];
    let find = |kind: &str| node.actions.iter().position(|a| a.kind == kind);
    match action {
        Action::Fold => find("fold"),
        Action::Check => find("check"),
        Action::Call => find("call"),
        Action::Bet { to } | Action::Raise { to } => {
            let all_in = to == p.stack + p.street_commit;
            if all_in {
                if let Some(j) = find("jam") {
                    return Some(j);
                }
            }
            let to_bb = to.mbb() as f64 / before.big_blind().mbb() as f64;
            node.actions
                .iter()
                .enumerate()
                .filter(|(_, a)| a.kind != "fold" && a.kind != "call" && a.kind != "check")
                .min_by(|(_, a), (_, b)| {
                    let d = |x: f64| (x / 1000.0 / to_bb).ln().abs();
                    d(a.to_mbb).total_cmp(&d(b.to_mbb))
                })
                .map(|(j, _)| j)
        }
    }
}

fn after_action(hand: &mut Hand, before: &GameState, seat: usize, action: Action, model_action: Option<usize>) {
    if before.board().cards().is_empty() {
        let next = match model_action {
            Some(a) => Some(a),
            None if hand.seat_player[seat] == 0 => translate(hand, before, seat, action),
            None => None,
        };
        hand.model_path = match (hand.model_path.take(), next) {
            (Some(mut path), Some(a)) => {
                path.push(a);
                Some(path)
            }
            _ => None,
        };
    }
    let rec = hand.state.history().last().copied().expect("action recorded");
    hand.record.actions.push(RecordAction {
        seat,
        street: street_name(rec.street).into(),
        action: rec.action,
        added: rec.added.mbb(),
        street_commit: rec.street_commit.mbb(),
        all_in: rec.all_in,
    });
}

/// Records what you faced before your action: pot odds, your equity against the bots' tracked
/// ranges, and (preflop) the model's frequencies for your hand.
fn hero_decision(hand: &mut Hand, seat: usize, action: Action) -> HeroDecision {
    let state = &hand.state;
    let legal = state.legal_actions().ok();
    let to_call = legal.and_then(|l| l.call).map_or(0, |c| c.mbb());
    let pot = state.pot_total().mbb();
    let board = state.board().cards().to_vec();
    let required = (to_call > 0).then(|| to_call as f64 / (pot + to_call) as f64);
    let opponents: Vec<usize> =
        (0..state.players().len()).filter(|&k| k != seat && !state.players()[k].folded).collect();
    // Computed once when your turn began (shown live), so the record matches what you saw.
    let equity = hand.hero_odds.and_then(|o| o.equity);
    let mut model_check = None;
    if board.is_empty() {
        if let (Some(model), Some(path)) = (&hand.model, &hand.model_path) {
            if let Some(node) = model.node(path) {
                if node.kind == "action" && node.actor.as_deref() == Some(state.players()[seat].position.short_name()) {
                    let [a, b] = hand.hole[seat].cards();
                    let class = range_engine::HandClass::of(Combo::new(a, b).expect("hole"));
                    if let Some(f) = node.by_class.iter().find(|c| c.label == class.to_string()) {
                        let taken = translate(hand, state, seat, action);
                        model_check = Some(ModelCheck {
                            spot: crate::record::spot_name(state, seat),
                            class: class.to_string(),
                            actions: node.actions.iter().map(|a| a.label.clone()).collect(),
                            frequencies: f.frequencies.clone(),
                            taken,
                        });
                    }
                }
            }
        }
    }
    HeroDecision {
        street: street_name(state.street()).into(),
        action,
        to_call,
        pot,
        required_equity: required,
        equity_vs_ranges: equity,
        opponents: opponents.len(),
        model: model_check,
        all_in_call: legal.and_then(|l| l.call).is_some_and(|c| c == state.players()[seat].stack),
        feedback: None,
        spot: crate::spot::capture(state, seat, action, |k| hand.street_ranges[k].clone(), hand.hero_range.clone()),
    }
}

/// Your range when the flop is dealt, from the preflop model's node (`None` off the model tree).
fn hero_model_range(hand: &Hand) -> Option<String> {
    let node = hand.model.as_ref()?.node(hand.model_path.as_ref()?)?;
    let hero = hand.seat_player.iter().position(|&p| p == 0)?;
    let pos = hand.state.players()[hero].position.short_name();
    let seat = node.ranges.iter().find(|r| r.position == pos)?;
    crate::spot::class_notation(seat.classes.iter().map(|c| (c.label.as_str(), c.frequencies[0])))
}

/// Your equity against every live opponent's tracked range (Monte Carlo, seeded).
fn equity_vs_ranges(hand: &Hand, seat: usize, opponents: &[usize], board: &[Card]) -> Option<f64> {
    let [a, b] = hand.hole[seat].cards();
    let mine = Combo::new(a, b)?;
    let mut ranges = vec![equity_engine::single_combo_range(mine)];
    for &o in opponents {
        let r = Range::from_weights(hand.ranges[o].iter().map(|w| w.clamp(0.0, 1.0)).collect()).ok()?;
        if r.is_empty() {
            return None;
        }
        ranges.push(r);
    }
    let b = poker_core::Board::new(board.to_vec()).ok()?;
    let seed = hand.record.hand_no.wrapping_mul(31) ^ hand.state.history().len() as u64;
    equity_engine::monte_carlo(&ranges, &b, card_set(&[]), 6_000, seed).ok().map(|r| r.equity[0])
}
