//! Independent validation of solver output (Milestone 2).
//!
//! Given a postflop game tree (river, turn or flop root, with chance nodes dealing the remaining
//! cards) and **any** solver's strategy profile, this crate computes each player's expected
//! value, best-response value and the profile's exploitability. It uses only RiverLab's own
//! validated evaluator and combo/card-removal code, so it shares no code with the solver under
//! test.
//!
//! Conventions: amounts are in chips (any unit, e.g. mbb). `put[p]` is everything player `p`
//! has committed, including their half of the starting pot. Any uncalled excess is returned
//! first, so the pot is `2 × min(put)` and each player's stake is `m = min(put)`. Payoffs are
//! net of that stake: the winner nets `pot − rake − m`, the loser `−m`, and a tie pays each
//! `(pot − rake)/2 − m`. Rake is `min(pot × fraction, cap)` and applies to every terminal
//! (the flop has been dealt).
//!
//! Chance nodes deal one card. For any pair of non-overlapping hands, exactly
//! `52 − |board| − 4` cards can come, so each card is weighted by that reciprocal and hands
//! holding the dealt card are removed below it. That is the exact conditional distribution.

mod fast;
pub mod push_fold;

use hand_evaluator::evaluate_mask;
use poker_core::{Card, CardSet};
use range_engine::Combo;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ValidationError {
    #[error("invalid game: {0}")]
    InvalidGame(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rake {
    pub fraction: f64,
    pub cap: Option<f64>,
}

impl Rake {
    pub const NONE: Rake = Rake { fraction: 0.0, cap: None };
    fn of(self, pot: f64) -> f64 {
        let r = pot * self.fraction;
        self.cap.map_or(r, |c| r.min(c))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameNode {
    /// `strategy` is action-major: `strategy[a * n + i]` is the probability that the acting
    /// player's combo `i` (index into its range) takes action `a`; `n` = range length. Rows of
    /// combos blocked by cards dealt on the way to this node are ignored.
    Decision {
        player: usize,
        children: Vec<usize>,
        strategy: Vec<f64>,
    },
    /// Deals one card. Must have exactly one child per card not yet on the board.
    Chance {
        children: Vec<(Card, usize)>,
    },
    Fold {
        folder: usize,
        put: [f64; 2],
    },
    /// Only valid once the board has five cards.
    Showdown {
        put: [f64; 2],
    },
}

/// A complete postflop game: starting board (3–5 cards), weighted ranges, rake and a tree whose
/// root is `nodes[0]`.
#[derive(Debug, Clone, PartialEq)]
pub struct PostflopGame {
    pub board: Vec<Card>,
    pub ranges: [Vec<(Combo, f64)>; 2],
    pub rake: Rake,
    pub nodes: Vec<GameNode>,
    /// Decision nodes whose strategy is fixed (node locking). A best response follows the given
    /// strategy there instead of choosing, so exploitability measures how far the rest of the
    /// profile is from answering the locks optimally. Empty for an ordinary solve.
    pub locked: std::collections::BTreeSet<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evaluation {
    /// Expected value per hand for each player when both follow the profile.
    pub ev: [f64; 2],
    /// Value per hand of a best response against the other player's profile strategy (following
    /// the profile at locked nodes).
    pub br: [f64; 2],
    /// `(br[0] − ev[0]) + (br[1] − ev[1])`. Zero exactly at a Nash equilibrium.
    pub nash_conv: f64,
    /// `nash_conv / 2`: the average gain available to a deviating player.
    pub exploitability: f64,
}

struct Reference<'a> {
    game: &'a PostflopGame,
    masks: [Vec<u64>; 2],
}

impl Reference<'_> {
    fn compat(&self, p: usize, i: usize, j: usize) -> bool {
        self.masks[p][i] & self.masks[1 - p][j] == 0
    }

    /// Counterfactual values for player `p`'s combos at `node`, given the opponent's reach and
    /// the full board so far. Combos blocked by `board` get 0.
    fn cfv(&self, node: usize, p: usize, reach_o: &[f64], board: u64, best: bool) -> Vec<f64> {
        let n = self.masks[p].len();
        let o = 1 - p;
        let live = |i: usize| self.masks[p][i] & board == 0;
        match &self.game.nodes[node] {
            GameNode::Fold { folder, put } => {
                let m = put[0].min(put[1]);
                let pot = 2.0 * m;
                let u = if *folder == p { -m } else { pot - self.game.rake.of(pot) - m };
                (0..n)
                    .map(|i| {
                        if !live(i) {
                            return 0.0;
                        }
                        u * (0..reach_o.len()).filter(|&j| self.compat(p, i, j)).map(|j| reach_o[j]).sum::<f64>()
                    })
                    .collect()
            }
            GameNode::Showdown { put } => {
                let m = put[0].min(put[1]);
                let pot = 2.0 * m;
                let net = pot - self.game.rake.of(pot);
                let (win, lose, tie) = (net - m, -m, net / 2.0 - m);
                let strength = |q: usize, k: usize| evaluate_mask(board | self.masks[q][k]).value();
                let opp: Vec<u32> =
                    (0..reach_o.len()).map(|j| if reach_o[j] > 0.0 { strength(o, j) } else { 0 }).collect();
                (0..n)
                    .map(|i| {
                        if !live(i) {
                            return 0.0;
                        }
                        let si = strength(p, i);
                        (0..reach_o.len())
                            .filter(|&j| reach_o[j] > 0.0 && self.compat(p, i, j))
                            .map(|j| {
                                reach_o[j]
                                    * if si > opp[j] {
                                        win
                                    } else if si < opp[j] {
                                        lose
                                    } else {
                                        tie
                                    }
                            })
                            .sum()
                    })
                    .collect()
            }
            GameNode::Chance { children } => {
                let w = 1.0 / (52 - board.count_ones() as usize - 4) as f64;
                let mut out = vec![0.0; n];
                for &(card, child) in children {
                    let bit = 1u64 << card.index();
                    let r: Vec<f64> = (0..reach_o.len())
                        .map(|j| if self.masks[o][j] & bit == 0 { reach_o[j] } else { 0.0 })
                        .collect();
                    let v = self.cfv(child, p, &r, board | bit, best);
                    for i in 0..n {
                        if self.masks[p][i] & bit == 0 {
                            out[i] += w * v[i];
                        }
                    }
                }
                out
            }
            GameNode::Decision { player, children, strategy } => {
                if *player == p {
                    let vals: Vec<Vec<f64>> = children.iter().map(|&c| self.cfv(c, p, reach_o, board, best)).collect();
                    (0..n)
                        .map(|i| {
                            if !live(i) {
                                0.0
                            } else if best && !self.game.locked.contains(&node) {
                                vals.iter().map(|v| v[i]).fold(f64::NEG_INFINITY, f64::max)
                            } else {
                                vals.iter().enumerate().map(|(a, v)| strategy[a * n + i] * v[i]).sum()
                            }
                        })
                        .collect()
                } else {
                    let m = reach_o.len();
                    let mut out = vec![0.0; n];
                    for (a, &c) in children.iter().enumerate() {
                        let r: Vec<f64> = (0..m).map(|j| reach_o[j] * strategy[a * m + j]).collect();
                        for (x, v) in out.iter_mut().zip(self.cfv(c, p, &r, board, best)) {
                            *x += v;
                        }
                    }
                    out
                }
            }
        }
    }
}

fn invalid<T>(m: String) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidGame(m))
}

fn validate_node(game: &PostflopGame, masks: &[Vec<u64>; 2], k: usize, board: u64) -> Result<(), ValidationError> {
    let len = game.nodes.len();
    match &game.nodes[k] {
        GameNode::Decision { player, children, strategy } => {
            let n = game
                .ranges
                .get(*player)
                .ok_or_else(|| ValidationError::InvalidGame(format!("node {k}: bad player")))?
                .len();
            if children.is_empty() || children.iter().any(|&c| c <= k || c >= len) {
                return invalid(format!("node {k}: children must point forward to existing nodes"));
            }
            if strategy.len() != children.len() * n {
                return invalid(format!(
                    "node {k}: strategy has {} entries, expected {}",
                    strategy.len(),
                    children.len() * n
                ));
            }
            for i in (0..n).filter(|&i| masks[*player][i] & board == 0) {
                let row = (0..children.len()).map(|a| strategy[a * n + i]);
                let s: f64 = row.clone().sum();
                if (s - 1.0).abs() > 1e-3 || row.clone().any(|x| !(-1e-9..=1.0 + 1e-9).contains(&x)) {
                    return invalid(format!("node {k}: combo {i} probabilities sum to {s}"));
                }
            }
            for &c in children {
                validate_node(game, masks, c, board)?;
            }
        }
        GameNode::Chance { children } => {
            if board.count_ones() >= 5 {
                return invalid(format!("node {k}: chance node after the river"));
            }
            let mut seen = 0u64;
            for &(card, child) in children {
                let bit = 1u64 << card.index();
                if board & bit != 0 || seen & bit != 0 {
                    return invalid(format!("node {k}: card {card} dealt twice or already on the board"));
                }
                if child <= k || child >= len {
                    return invalid(format!("node {k}: children must point forward to existing nodes"));
                }
                seen |= bit;
                validate_node(game, masks, child, board | bit)?;
            }
            if (seen | board).count_ones() != 52 {
                return invalid(format!("node {k}: chance node must deal every remaining card"));
            }
        }
        GameNode::Showdown { .. } if board.count_ones() != 5 => {
            return invalid(format!("node {k}: showdown before the river"));
        }
        GameNode::Showdown { .. } | GameNode::Fold { .. } => {}
    }
    Ok(())
}

fn validate(game: &PostflopGame, masks: &[Vec<u64>; 2]) -> Result<u64, ValidationError> {
    if !(3..=5).contains(&game.board.len()) {
        return invalid(format!("board must have 3 to 5 cards, got {}", game.board.len()));
    }
    let board = CardSet::from_distinct(&game.board)
        .map_err(|c| ValidationError::InvalidGame(format!("duplicate board card {c}")))?;
    for (p, range) in game.ranges.iter().enumerate() {
        let mut seen = vec![false; range_engine::COMBO_COUNT];
        for &(c, w) in range {
            if !(w.is_finite() && w > 0.0) {
                return invalid(format!("player {p} combo {c} has weight {w}"));
            }
            if c.card_set().intersects(board) {
                return invalid(format!("player {p} combo {c} uses a board card"));
            }
            if std::mem::replace(&mut seen[c.index()], true) {
                return invalid(format!("player {p} lists combo {c} twice"));
            }
        }
    }
    if game.nodes.is_empty() {
        return invalid("empty tree".into());
    }
    validate_node(game, masks, 0, board.bits())?;
    Ok(board.bits())
}

/// Evaluates a strategy profile on a postflop game (fast path: sorted showdown sweeps with
/// per-card inclusion–exclusion, chance subtrees in parallel). Cross-checked against
/// [`evaluate_reference`] by property tests.
pub fn evaluate(game: &PostflopGame) -> Result<Evaluation, ValidationError> {
    let masks: [Vec<u64>; 2] =
        std::array::from_fn(|p| game.ranges[p].iter().map(|(c, _)| c.card_set().bits()).collect());
    let board = validate(game, &masks)?;
    let fast = fast::Fast::new(game, &masks);
    let weights: [Vec<f64>; 2] = std::array::from_fn(|p| game.ranges[p].iter().map(|&(_, w)| w).collect());
    let z: f64 = (0..weights[0].len()).map(|i| weights[0][i] * fast.live_mass(0, i, &weights[1])).sum();
    if z <= 0.0 {
        return invalid("no compatible pair of hands".into());
    }
    // Each player's profile value and best-response value come from one traversal; the two
    // players are traversed in parallel.
    let values = |p: usize| -> (f64, f64) {
        let (ev, br) = fast.cfv_both(0, p, &weights[1 - p], board);
        let sum = |v: &[f64]| weights[p].iter().zip(v).map(|(w, x)| w * x).sum::<f64>() / z;
        (sum(&ev), sum(&br))
    };
    let ((ev0, br0), (ev1, br1)) = rayon::join(|| values(0), || values(1));
    let ev = [ev0, ev1];
    let br = [br0, br1];
    let nash_conv = (br[0] - ev[0]) + (br[1] - ev[1]);
    Ok(Evaluation { ev, br, nash_conv, exploitability: nash_conv / 2.0 })
}

/// The original, deliberately simple implementation: O(n·m) pair checks at every terminal.
/// Slow on wide flop ranges, but obviously correct; kept as the oracle for [`evaluate`].
pub fn evaluate_reference(game: &PostflopGame) -> Result<Evaluation, ValidationError> {
    let masks: [Vec<u64>; 2] =
        std::array::from_fn(|p| game.ranges[p].iter().map(|(c, _)| c.card_set().bits()).collect());
    let board = validate(game, &masks)?;
    let prep = Reference { game, masks };
    let weights: [Vec<f64>; 2] = std::array::from_fn(|p| game.ranges[p].iter().map(|&(_, w)| w).collect());
    let z: f64 = (0..weights[0].len())
        .map(|i| {
            weights[0][i] * (0..weights[1].len()).filter(|&j| prep.compat(0, i, j)).map(|j| weights[1][j]).sum::<f64>()
        })
        .sum();
    if z <= 0.0 {
        return invalid("no compatible pair of hands".into());
    }
    let value = |p: usize, best: bool| -> f64 {
        let cfv = prep.cfv(0, p, &weights[1 - p], board, best);
        weights[p].iter().zip(&cfv).map(|(w, v)| w * v).sum::<f64>() / z
    };
    let ev = [value(0, false), value(1, false)];
    let br = [value(0, true), value(1, true)];
    let nash_conv = (br[0] - ev[0]) + (br[1] - ev[1]);
    Ok(Evaluation { ev, br, nash_conv, exploitability: nash_conv / 2.0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use range_engine::Range;

    /// Clairvoyance-style spot solved by hand (docs/solver-comparison.md): board KsQd7h7c2s,
    /// OOP 12 AQ (checks), IP 3 KK + 12 43o; pot 10, IP may bet 10 (all-in); OOP calls/folds.
    fn toy(bluff: f64, call: f64, rake: Rake) -> PostflopGame {
        let board: [Card; 5] = Card::parse_many("KsQd7h7c2s").unwrap().try_into().unwrap();
        let dead = CardSet::from_cards(board);
        let live = |s: &str| -> Vec<(Combo, f64)> { Range::parse(s).unwrap().without_cards(dead).iter().collect() };
        let oop = live("AQ");
        let ip = live("KK,43o");
        let is_kk = |c: &Combo| c.hole_cards().high().rank() == poker_core::Rank::King;
        let n_ip = ip.len();
        // IP node after check: actions [check, bet]
        let mut ip_strat = vec![0.0; 2 * n_ip];
        for (i, (c, _)) in ip.iter().enumerate() {
            let b = if is_kk(c) { 1.0 } else { bluff };
            ip_strat[i] = 1.0 - b;
            ip_strat[n_ip + i] = b;
        }
        let n_oop = oop.len();
        let oop_call: Vec<f64> = (0..n_oop).map(|_| 1.0 - call).chain((0..n_oop).map(|_| call)).collect();
        PostflopGame {
            board: board.to_vec(),
            ranges: [oop.clone(), ip],
            rake,
            nodes: vec![
                GameNode::Decision { player: 0, children: vec![1], strategy: vec![1.0; n_oop] },
                GameNode::Decision { player: 1, children: vec![2, 3], strategy: ip_strat },
                GameNode::Showdown { put: [5.0, 5.0] },
                GameNode::Decision { player: 0, children: vec![4, 5], strategy: oop_call },
                GameNode::Fold { folder: 0, put: [5.0, 15.0] },
                GameNode::Showdown { put: [15.0, 15.0] },
            ],
            locked: Default::default(),
        }
    }

    #[test]
    fn locked_nodes_are_constraints_for_the_best_response() {
        // IP bluffs every time and OOP always calls: OOP's call is the best response to that,
        // but IP could gain by bluffing less. Locking IP's decision leaves nothing to gain.
        let free = toy(1.0, 1.0, Rake::NONE);
        assert!(evaluate(&free).unwrap().exploitability > 0.1);
        let mut locked = free.clone();
        locked.locked.insert(1);
        for e in [evaluate(&locked).unwrap(), evaluate_reference(&locked).unwrap()] {
            assert!(e.exploitability.abs() < 1e-9, "{e:?}");
            assert_eq!(e.ev, evaluate(&free).unwrap().ev, "locks change best responses, not values");
        }
        // With OOP folding instead, OOP's own response is wrong, and the lock does not hide it.
        let mut bad = toy(1.0, 0.0, Rake::NONE);
        bad.locked.insert(1);
        assert!(evaluate(&bad).unwrap().exploitability > 0.1);
    }

    #[test]
    fn hand_solved_equilibrium_has_zero_exploitability() {
        let e = evaluate(&toy(1.5 / 12.0, 0.5, Rake::NONE)).unwrap();
        assert!(e.exploitability.abs() < 1e-12, "{e:?}");
        // IP pot share 3 => net 3 - 5 = -2.
        assert!((e.ev[1] + 2.0).abs() < 1e-12, "{e:?}");
        assert!((e.ev[0] - 2.0).abs() < 1e-12, "zero-sum without rake: {e:?}");
    }

    #[test]
    fn raked_equilibrium_has_zero_exploitability() {
        let rake = Rake { fraction: 0.05, cap: None };
        // OOP indifferent when bluff/value = 10/18.5 (3 value combos).
        // IP's air indifferent when c·(−15) + (1−c)·4.5 = −5, i.e. call c = 9.5/19.5.
        let e = evaluate(&toy(3.0 * 10.0 / 18.5 / 12.0, 9.5 / 19.5, rake)).unwrap();
        assert!(e.exploitability.abs() < 1e-12, "{e:?}");
        assert!(e.ev[0] + e.ev[1] < 0.0, "rake makes the game negative-sum");
    }

    #[test]
    fn deviations_are_exploitable_by_exactly_the_right_amount() {
        // OOP always calls: IP's best response stops bluffing. IP gains 1.5 bluff combos * 10 / 15 combos = 1.0.
        let e = evaluate(&toy(1.5 / 12.0, 1.0, Rake::NONE)).unwrap();
        assert!((e.br[1] - e.ev[1] - 1.0).abs() < 1e-12, "{e:?}");
        assert!(e.exploitability > 0.4);
        // IP never bluffs: OOP's best response folds; OOP gains 0.5 * 3 * 10 / 15 = 1.0.
        let e = evaluate(&toy(0.0, 0.5, Rake::NONE)).unwrap();
        assert!((e.br[0] - e.ev[0] - 1.0).abs() < 1e-12, "{e:?}");
    }

    #[test]
    fn uncalled_bets_are_returned_before_rake() {
        // IP bets 10, OOP folds: pot is the matched 10, rake 0.5, IP nets 4.5.
        let rake = Rake { fraction: 0.05, cap: None };
        let mut g = toy(0.0, 0.0, rake);
        g.ranges[1].retain(|(c, _)| c.hole_cards().high().rank() == poker_core::Rank::King);
        let n = g.ranges[1].len();
        g.nodes[1] =
            GameNode::Decision { player: 1, children: vec![2, 3], strategy: [vec![0.0; n], vec![1.0; n]].concat() };
        let e = evaluate(&g).unwrap();
        assert!((e.ev[1] - 4.5).abs() < 1e-12, "{e:?}");
    }

    /// Turn with no betting: every pair reaches showdown, so values must equal
    /// `pot × equity − stake`, with equity from the (independently validated) equity engine.
    fn check_down(board: &str, ranges: [&str; 2]) -> PostflopGame {
        let board = Card::parse_many(board).unwrap();
        let dead = CardSet::from_cards(board.iter().copied());
        let r = |s: &str| -> Vec<(Combo, f64)> { Range::parse(s).unwrap().without_cards(dead).iter().collect() };
        let remaining: Vec<Card> = Card::all().filter(|c| !dead.contains(*c)).collect();
        let mut nodes = vec![GameNode::Chance { children: Vec::new() }];
        let mut children = Vec::new();
        for c in remaining {
            children.push((c, nodes.len()));
            nodes.push(GameNode::Showdown { put: [5.0, 5.0] });
        }
        nodes[0] = GameNode::Chance { children };
        PostflopGame {
            board,
            ranges: [r(ranges[0]), r(ranges[1])],
            rake: Rake::NONE,
            nodes,
            locked: Default::default(),
        }
    }

    #[test]
    fn chance_nodes_reproduce_exact_equity_with_card_removal() {
        for (board, ranges) in
            [("Ks7d2c8h", ["AA,KQ,76s:0.5", "QQ-22,AJs+,KJo+"]), ("Qh7h2s3d", ["AhKh,JhTh,QsJs", "77,22,AQ"])]
        {
            let g = check_down(board, ranges);
            let e = evaluate(&g).unwrap();
            let rs: Vec<range_engine::Range> = ranges.iter().map(|s| Range::parse(s).unwrap()).collect();
            let b: poker_core::Board = board.parse().unwrap();
            let eq = equity_engine::exact(&rs, &b, CardSet::EMPTY, u64::MAX).unwrap();
            for p in 0..2 {
                assert!(
                    (e.ev[p] - (10.0 * eq.equity[p] - 5.0)).abs() < 1e-9,
                    "{board} p{p}: {} vs {}",
                    e.ev[p],
                    eq.equity[p]
                );
            }
            assert!(e.exploitability.abs() < 1e-9, "no decisions, nothing to exploit");
        }
    }

    #[test]
    fn chance_nodes_must_deal_every_remaining_card() {
        let mut g = check_down("Ks7d2c8h", ["AA", "KK"]);
        if let GameNode::Chance { children } = &mut g.nodes[0] {
            children.pop();
        }
        assert!(evaluate(&g).is_err());
        let mut g = check_down("Ks7d2c8h", ["AA", "KK"]);
        g.nodes.push(GameNode::Showdown { put: [5.0, 5.0] });
        g.board.pop();
        assert!(evaluate(&g).is_err(), "showdown on a 4-card board path must be rejected");
    }

    #[test]
    fn malformed_profiles_are_rejected() {
        let mut g = toy(0.1, 0.5, Rake::NONE);
        if let GameNode::Decision { strategy, .. } = &mut g.nodes[3] {
            strategy[0] = 0.9;
        }
        assert!(evaluate(&g).is_err());
        let mut g = toy(0.1, 0.5, Rake::NONE);
        g.ranges[0].push(g.ranges[0][0]);
        assert!(evaluate(&g).is_err());
    }
}
