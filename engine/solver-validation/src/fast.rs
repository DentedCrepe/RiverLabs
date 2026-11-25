//! Fast evaluation: the same quantities as the reference implementation, computed with
//! per-card inclusion–exclusion instead of explicit pair checks.
//!
//! For a hero combo with cards `a`, `b`, the opponent reach compatible with it is
//! `total − card[a] − card[b] + same`, where `card[c]` sums the reach of opponent combos holding
//! `c` and `same` is the reach of the opponent combo identical to the hero's (subtracted twice).
//! Showdowns sweep both players' combos in strength order, keeping the same sums for the
//! opponents strictly below and exactly equal to the hero's strength.

use crate::{GameNode, PostflopGame};
use hand_evaluator::evaluate_mask;
use rayon::prelude::*;

pub(crate) struct Fast<'a> {
    game: &'a PostflopGame,
    masks: [&'a [u64]; 2],
    /// Card indices of each combo.
    cards: [Vec<(usize, usize)>; 2],
    /// For each combo of player p, the index of the identical combo in the other player's range.
    same: [Vec<Option<usize>>; 2],
    /// For each five-card board that reaches a showdown, each player's combos that do not
    /// overlap it, as (hand strength, index) in ascending strength. Computed once, instead of at
    /// every showdown node on that board.
    ranked: std::collections::HashMap<u64, [Vec<(u32, usize)>; 2]>,
}

/// The five-card boards at which the tree has a showdown.
fn showdown_boards(game: &PostflopGame, node: usize, board: u64, out: &mut std::collections::HashSet<u64>) {
    match &game.nodes[node] {
        GameNode::Showdown { .. } => {
            out.insert(board);
        }
        GameNode::Fold { .. } => {}
        GameNode::Chance { children } => {
            for &(card, child) in children {
                showdown_boards(game, child, board | 1u64 << card.index(), out);
            }
        }
        GameNode::Decision { children, .. } => {
            for &child in children {
                showdown_boards(game, child, board, out);
            }
        }
    }
}

/// Per-combo values when following the profile and when best-responding.
pub(crate) type Both = (Vec<f64>, Vec<f64>);

fn card_pair(m: u64) -> (usize, usize) {
    let a = m.trailing_zeros() as usize;
    let b = (m & (m - 1)).trailing_zeros() as usize;
    (a, b)
}

/// Per-card sums of opponent reach.
struct Sums {
    total: f64,
    card: [f64; 52],
}

impl Sums {
    fn new() -> Sums {
        Sums { total: 0.0, card: [0.0; 52] }
    }
    fn add(&mut self, (a, b): (usize, usize), r: f64) {
        self.total += r;
        self.card[a] += r;
        self.card[b] += r;
    }
    fn excluding(&self, (a, b): (usize, usize)) -> f64 {
        self.total - self.card[a] - self.card[b]
    }
}

impl<'a> Fast<'a> {
    pub(crate) fn new(game: &'a PostflopGame, masks: &'a [Vec<u64>; 2]) -> Fast<'a> {
        let cards = std::array::from_fn(|p| masks[p].iter().map(|&m| card_pair(m)).collect());
        let same = std::array::from_fn(|p| {
            let mut index = vec![usize::MAX; 1326];
            for (j, (c, _)) in game.ranges[1 - p].iter().enumerate() {
                index[c.index()] = j;
            }
            game.ranges[p].iter().map(|(c, _)| Some(index[c.index()]).filter(|&j| j != usize::MAX)).collect()
        });
        let mut boards = std::collections::HashSet::new();
        let start = game.board.iter().fold(0u64, |m, c| m | 1u64 << c.index());
        if !game.nodes.is_empty() {
            showdown_boards(game, 0, start, &mut boards);
        }
        let ranked = boards
            .into_par_iter()
            .map(|board| {
                let rank = |p: usize| {
                    let mut v: Vec<(u32, usize)> = (0..masks[p].len())
                        .filter(|&i| masks[p][i] & board == 0)
                        .map(|i| (evaluate_mask(board | masks[p][i]).value(), i))
                        .collect();
                    v.sort_unstable();
                    v
                };
                (board, [rank(0), rank(1)])
            })
            .collect();
        Fast { game, masks: [&masks[0], &masks[1]], cards, same, ranked }
    }

    /// Opponent reach compatible with hero combo `i` (for the normaliser).
    pub(crate) fn live_mass(&self, p: usize, i: usize, reach_o: &[f64]) -> f64 {
        let o = 1 - p;
        let mut s = Sums::new();
        for (j, &r) in reach_o.iter().enumerate() {
            s.add(self.cards[o][j], r);
        }
        s.excluding(self.cards[p][i]) + self.same[p][i].map_or(0.0, |j| reach_o[j])
    }

    /// Values for player `p`'s combos at a fold or showdown node.
    fn terminal_cfv(&self, node: usize, p: usize, reach_o: &[f64], board: u64) -> Vec<f64> {
        let n = self.masks[p].len();
        let o = 1 - p;
        let live = |i: usize| self.masks[p][i] & board == 0;
        match &self.game.nodes[node] {
            GameNode::Fold { folder, put } => {
                let m = put[0].min(put[1]);
                let pot = 2.0 * m;
                let u = if *folder == p { -m } else { pot - self.game.rake.of(pot) - m };
                let mut s = Sums::new();
                for (j, &r) in reach_o.iter().enumerate() {
                    if r > 0.0 {
                        s.add(self.cards[o][j], r);
                    }
                }
                (0..n)
                    .map(|i| {
                        if !live(i) {
                            return 0.0;
                        }
                        let same = self.same[p][i].map_or(0.0, |j| reach_o[j]);
                        u * (s.excluding(self.cards[p][i]) + same)
                    })
                    .collect()
            }
            GameNode::Showdown { put } => {
                let m = put[0].min(put[1]);
                let pot = 2.0 * m;
                let net = pot - self.game.rake.of(pot);
                let (win, lose, tie) = (net - m, -m, net / 2.0 - m);
                // Opponents with reach, and the hero's live combos, sorted by strength.
                let ranked = &self.ranked[&board];
                let opp: Vec<(u32, usize)> = ranked[o].iter().copied().filter(|&(_, j)| reach_o[j] > 0.0).collect();
                let hero = &ranked[p];
                let mut all = Sums::new();
                for &(_, j) in &opp {
                    all.add(self.cards[o][j], reach_o[j]);
                }
                let mut below = Sums::new();
                let mut out = vec![0.0; n];
                let (mut k, mut h) = (0usize, 0usize);
                while h < hero.len() {
                    let s = hero[h].0;
                    while k < opp.len() && opp[k].0 < s {
                        below.add(self.cards[o][opp[k].1], reach_o[opp[k].1]);
                        k += 1;
                    }
                    let mut equal = Sums::new();
                    let mut e = k;
                    while e < opp.len() && opp[e].0 == s {
                        equal.add(self.cards[o][opp[e].1], reach_o[opp[e].1]);
                        e += 1;
                    }
                    while h < hero.len() && hero[h].0 == s {
                        let i = hero[h].1;
                        let c = self.cards[p][i];
                        // An identical opponent combo has the same strength: it is never below,
                        // always in the equal group.
                        let same = self.same[p][i].map_or(0.0, |j| reach_o[j]);
                        let n_below = below.excluding(c);
                        let n_equal = equal.excluding(c) + same;
                        let n_live = all.excluding(c) + same;
                        let n_above = n_live - n_below - n_equal;
                        out[i] = win * n_below + tie * n_equal + lose * n_above;
                        h += 1;
                    }
                }
                out
            }
            GameNode::Chance { .. } | GameNode::Decision { .. } => {
                unreachable!("terminal_cfv is only called on fold and showdown nodes")
            }
        }
    }

    /// Counterfactual values of player `p`'s combos at `node`, given the opponent's reach: when
    /// both players follow the profile (`.0`) and when `p` best-responds (`.1`, following the
    /// profile at locked nodes). Both come from one traversal: the opponent's reach, and so every
    /// showdown sweep, is the same for the two.
    pub(crate) fn cfv_both(&self, node: usize, p: usize, reach_o: &[f64], board: u64) -> Both {
        let n = self.masks[p].len();
        let o = 1 - p;
        let live = |i: usize| self.masks[p][i] & board == 0;
        match &self.game.nodes[node] {
            GameNode::Fold { .. } | GameNode::Showdown { .. } => {
                let v = self.terminal_cfv(node, p, reach_o, board);
                (v.clone(), v)
            }
            GameNode::Chance { children } => {
                let w = 1.0 / (52 - board.count_ones() as usize - 4) as f64;
                let parts: Vec<(u64, Both)> = children
                    .par_iter()
                    .map(|&(card, child)| {
                        let bit = 1u64 << card.index();
                        let r: Vec<f64> = (0..reach_o.len())
                            .map(|j| if self.masks[o][j] & bit == 0 { reach_o[j] } else { 0.0 })
                            .collect();
                        (bit, self.cfv_both(child, p, &r, board | bit))
                    })
                    .collect();
                let (mut ev, mut br) = (vec![0.0; n], vec![0.0; n]);
                for (bit, (e, b)) in parts {
                    for i in 0..n {
                        if self.masks[p][i] & bit == 0 {
                            ev[i] += w * e[i];
                            br[i] += w * b[i];
                        }
                    }
                }
                (ev, br)
            }
            GameNode::Decision { player, children, strategy } => {
                if *player == p {
                    let vals: Vec<Both> = children.iter().map(|&c| self.cfv_both(c, p, reach_o, board)).collect();
                    let locked = self.game.locked.contains(&node);
                    let mut ev = vec![0.0; n];
                    let mut br = vec![0.0; n];
                    for i in (0..n).filter(|&i| live(i)) {
                        ev[i] = vals.iter().enumerate().map(|(a, v)| strategy[a * n + i] * v.0[i]).sum();
                        br[i] = if locked {
                            vals.iter().enumerate().map(|(a, v)| strategy[a * n + i] * v.1[i]).sum()
                        } else {
                            vals.iter().map(|v| v.1[i]).fold(f64::NEG_INFINITY, f64::max)
                        };
                    }
                    (ev, br)
                } else {
                    let m = reach_o.len();
                    let (mut ev, mut br) = (vec![0.0; n], vec![0.0; n]);
                    for (a, &c) in children.iter().enumerate() {
                        let r: Vec<f64> = (0..m).map(|j| reach_o[j] * strategy[a * m + j]).collect();
                        let (e, b) = self.cfv_both(c, p, &r, board);
                        for i in 0..n {
                            ev[i] += e[i];
                            br[i] += b[i];
                        }
                    }
                    (ev, br)
                }
            }
        }
    }
}
