//! Independent validation of solver output (Milestone 2).
//!
//! Given a river game tree and **any** solver's strategy profile, this crate computes each
//! player's expected value, best-response value and the profile's exploitability. It uses
//! only RiverLab's own validated evaluator and combo/card-removal code, so it shares no
//! code with the solver under test.
//!
//! Conventions: amounts are in chips (any unit, e.g. mbb). `put[p]` is everything player `p`
//! has committed, including their half of the starting pot. Any uncalled excess is returned
//! first, so the pot is `2 × min(put)` and each player's stake is `m = min(put)`. Payoffs are
//! net of that stake: the winner nets `pot − rake − m`, the loser `−m`, and a tie pays each
//! `(pot − rake)/2 − m`. Rake is `min(pot × fraction, cap)` and applies to every terminal
//! (the flop has been dealt).

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
pub enum RiverNode {
    /// `strategy` is action-major: `strategy[a * n + i]` is the probability that the acting
    /// player's combo `i` (index into its range) takes action `a`; `n` = range length.
    Decision {
        player: usize,
        children: Vec<usize>,
        strategy: Vec<f64>,
    },
    Fold {
        folder: usize,
        put: [f64; 2],
    },
    Showdown {
        put: [f64; 2],
    },
}

/// A complete river game: board, weighted ranges, rake and a tree whose root is `nodes[0]`.
#[derive(Debug, Clone, PartialEq)]
pub struct RiverGame {
    pub board: [Card; 5],
    pub ranges: [Vec<(Combo, f64)>; 2],
    pub rake: Rake,
    pub nodes: Vec<RiverNode>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evaluation {
    /// Expected value per hand for each player when both follow the profile.
    pub ev: [f64; 2],
    /// Value per hand of a best response against the other player's profile strategy.
    pub br: [f64; 2],
    /// `(br[0] − ev[0]) + (br[1] − ev[1])`. Zero exactly at a Nash equilibrium.
    pub nash_conv: f64,
    /// `nash_conv / 2`: the average gain available to a deviating player.
    pub exploitability: f64,
}

struct Prepared<'a> {
    game: &'a RiverGame,
    masks: [Vec<u64>; 2],
    strength: [Vec<u32>; 2],
}

impl Prepared<'_> {
    fn compat(&self, p: usize, i: usize, j: usize) -> bool {
        self.masks[p][i] & self.masks[1 - p][j] == 0
    }

    /// Counterfactual values for player `p`'s combos at `node`, given the opponent's reach.
    fn cfv(&self, node: usize, p: usize, reach_o: &[f64], best: bool) -> Vec<f64> {
        let n = self.masks[p].len();
        let o = 1 - p;
        match &self.game.nodes[node] {
            RiverNode::Fold { folder, put } => {
                let m = put[0].min(put[1]);
                let pot = 2.0 * m;
                let u = if *folder == p { -m } else { pot - self.game.rake.of(pot) - m };
                (0..n)
                    .map(|i| u * (0..reach_o.len()).filter(|&j| self.compat(p, i, j)).map(|j| reach_o[j]).sum::<f64>())
                    .collect()
            }
            RiverNode::Showdown { put } => {
                let m = put[0].min(put[1]);
                let pot = 2.0 * m;
                let net = pot - self.game.rake.of(pot);
                let (win, lose, tie) = (net - m, -m, net / 2.0 - m);
                (0..n)
                    .map(|i| {
                        let si = self.strength[p][i];
                        (0..reach_o.len())
                            .filter(|&j| self.compat(p, i, j))
                            .map(|j| {
                                let sj = self.strength[o][j];
                                reach_o[j]
                                    * if si > sj {
                                        win
                                    } else if si < sj {
                                        lose
                                    } else {
                                        tie
                                    }
                            })
                            .sum()
                    })
                    .collect()
            }
            RiverNode::Decision { player, children, strategy } => {
                if *player == p {
                    let vals: Vec<Vec<f64>> = children.iter().map(|&c| self.cfv(c, p, reach_o, best)).collect();
                    (0..n)
                        .map(|i| {
                            if best {
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
                        for (x, v) in out.iter_mut().zip(self.cfv(c, p, &r, best)) {
                            *x += v;
                        }
                    }
                    out
                }
            }
        }
    }
}

fn validate(game: &RiverGame) -> Result<(), ValidationError> {
    let bad = |m: String| Err(ValidationError::InvalidGame(m));
    let board = CardSet::from_distinct(&game.board)
        .map_err(|c| ValidationError::InvalidGame(format!("duplicate board card {c}")))?;
    for (p, range) in game.ranges.iter().enumerate() {
        let mut seen = vec![false; range_engine::COMBO_COUNT];
        for &(c, w) in range {
            if !(w.is_finite() && w > 0.0) {
                return bad(format!("player {p} combo {c} has weight {w}"));
            }
            if c.card_set().intersects(board) {
                return bad(format!("player {p} combo {c} uses a board card"));
            }
            if std::mem::replace(&mut seen[c.index()], true) {
                return bad(format!("player {p} lists combo {c} twice"));
            }
        }
    }
    if game.nodes.is_empty() {
        return bad("empty tree".into());
    }
    for (k, node) in game.nodes.iter().enumerate() {
        if let RiverNode::Decision { player, children, strategy } = node {
            let n = game
                .ranges
                .get(*player)
                .ok_or_else(|| ValidationError::InvalidGame(format!("node {k}: bad player")))?
                .len();
            if children.is_empty() || children.iter().any(|&c| c <= k || c >= game.nodes.len()) {
                return bad(format!("node {k}: children must point forward to existing nodes"));
            }
            if strategy.len() != children.len() * n {
                return bad(format!(
                    "node {k}: strategy has {} entries, expected {}",
                    strategy.len(),
                    children.len() * n
                ));
            }
            for i in 0..n {
                let s: f64 = (0..children.len()).map(|a| strategy[a * n + i]).sum();
                if (s - 1.0).abs() > 1e-3
                    || (0..children.len()).any(|a| !(0.0..=1.0 + 1e-9).contains(&strategy[a * n + i]))
                {
                    return bad(format!("node {k}: combo {i} probabilities sum to {s}"));
                }
            }
        }
    }
    Ok(())
}

/// Evaluates a strategy profile on a river game.
pub fn evaluate(game: &RiverGame) -> Result<Evaluation, ValidationError> {
    validate(game)?;
    let board = CardSet::from_cards(game.board).bits();
    let masks: [Vec<u64>; 2] =
        std::array::from_fn(|p| game.ranges[p].iter().map(|(c, _)| c.card_set().bits()).collect());
    let strength: [Vec<u32>; 2] =
        std::array::from_fn(|p| masks[p].iter().map(|&m| evaluate_mask(board | m).value()).collect());
    let prep = Prepared { game, masks, strength };
    let weights: [Vec<f64>; 2] = std::array::from_fn(|p| game.ranges[p].iter().map(|&(_, w)| w).collect());
    let z: f64 = (0..weights[0].len())
        .map(|i| {
            weights[0][i] * (0..weights[1].len()).filter(|&j| prep.compat(0, i, j)).map(|j| weights[1][j]).sum::<f64>()
        })
        .sum();
    if z <= 0.0 {
        return Err(ValidationError::InvalidGame("no compatible pair of hands".into()));
    }
    let value = |p: usize, best: bool| -> f64 {
        let cfv = prep.cfv(0, p, &weights[1 - p], best);
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
    fn toy(bluff: f64, call: f64, rake: Rake) -> RiverGame {
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
        RiverGame {
            board,
            ranges: [oop.clone(), ip],
            rake,
            nodes: vec![
                RiverNode::Decision { player: 0, children: vec![1], strategy: vec![1.0; n_oop] },
                RiverNode::Decision { player: 1, children: vec![2, 3], strategy: ip_strat },
                RiverNode::Showdown { put: [5.0, 5.0] },
                RiverNode::Decision { player: 0, children: vec![4, 5], strategy: oop_call },
                RiverNode::Fold { folder: 0, put: [5.0, 15.0] },
                RiverNode::Showdown { put: [15.0, 15.0] },
            ],
        }
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
            RiverNode::Decision { player: 1, children: vec![2, 3], strategy: [vec![0.0; n], vec![1.0; n]].concat() };
        let e = evaluate(&g).unwrap();
        assert!((e.ev[1] - 4.5).abs() < 1e-12, "{e:?}");
    }

    #[test]
    fn malformed_profiles_are_rejected() {
        let mut g = toy(0.1, 0.5, Rake::NONE);
        if let RiverNode::Decision { strategy, .. } = &mut g.nodes[3] {
            strategy[0] = 0.9;
        }
        assert!(evaluate(&g).is_err());
        let mut g = toy(0.1, 0.5, Rake::NONE);
        g.ranges[0].push(g.ranges[0][0]);
        assert!(evaluate(&g).is_err());
    }
}
