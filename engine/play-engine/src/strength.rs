//! Hand strength used by the rule-based parts of the bots: Monte Carlo equity against one
//! uniformly random opponent hand ("strength vs a random hand"), with a seeded RNG so results are
//! reproducible. This is a standard heuristic: it rewards made hands and draws, but it is not a
//! solver evaluation.

use hand_evaluator::evaluate_mask;
use poker_core::{Card, CardSet};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use range_engine::{COMBO_COUNT, Combo, HandClass};
use std::sync::OnceLock;

fn mask(cards: &[Card]) -> u64 {
    cards.iter().fold(0u64, |m, c| m | (1u64 << c.index()))
}

/// Strength of `combo` on `board` (0–5 cards): equity against a random hand over random
/// run-outs, from `samples` draws.
pub fn strength_vs_random(combo: Combo, board: &[Card], dead: CardSet, samples: u32, seed: u64) -> f64 {
    let (a, b) = combo.cards();
    let used = mask(board) | (1u64 << a.index()) | (1u64 << b.index()) | dead_mask(dead);
    let deck: Vec<u8> = (0..52u8).filter(|i| used & (1u64 << i) == 0).collect();
    let need = 2 + (5 - board.len());
    if deck.len() < need || samples == 0 {
        return 0.5;
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ (combo.index() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let base = mask(board);
    let mine = (1u64 << a.index()) | (1u64 << b.index());
    let mut pool = deck.clone();
    let mut total = 0.0;
    for _ in 0..samples {
        // Partial Fisher–Yates: the first `need` entries become the opponent's cards and the run-out.
        for i in 0..need {
            let j = rng.random_range(i..pool.len());
            pool.swap(i, j);
        }
        let opp = (1u64 << pool[0]) | (1u64 << pool[1]);
        let run = pool[2..need].iter().fold(0u64, |m, &c| m | (1u64 << c));
        let full = base | run;
        let (h, o) = (evaluate_mask(full | mine), evaluate_mask(full | opp));
        total += match h.cmp(&o) {
            std::cmp::Ordering::Greater => 1.0,
            std::cmp::Ordering::Equal => 0.5,
            std::cmp::Ordering::Less => 0.0,
        };
    }
    total / samples as f64
}

pub fn card_set(cards: &[Card]) -> CardSet {
    let mut set = CardSet::EMPTY;
    for c in cards {
        set.insert(*c);
    }
    set
}

fn dead_mask(dead: CardSet) -> u64 {
    Card::all().filter(|c| dead.contains(*c)).fold(0u64, |m, c| m | (1u64 << c.index()))
}

/// Strength of every combo with positive weight in `weights` (others are 0).
pub fn range_strengths(weights: &[f64], board: &[Card], samples: u32, seed: u64) -> Vec<f64> {
    let board_set = card_set(board);
    let mut out = vec![0.0; COMBO_COUNT];
    for (i, w) in weights.iter().enumerate() {
        if *w > 0.0 {
            let c = Combo::from_index(i).expect("combo index");
            out[i] = strength_vs_random(c, board, board_set, samples, seed);
        }
    }
    out
}

/// Preflop strength of each of the 169 classes (equity vs a random hand), computed once per
/// process from 4,000 seeded samples per class. Indexed by `HandClass::grid_position`.
pub fn preflop_class_strength(class: HandClass) -> f64 {
    static TABLE: OnceLock<Vec<f64>> = OnceLock::new();
    let t = TABLE.get_or_init(|| {
        let mut t = vec![0.0; 169];
        for c in HandClass::all() {
            let combo = c.combos()[0];
            let (r, col) = c.grid_position();
            t[r * 13 + col] = strength_vs_random(combo, &[], CardSet::EMPTY, 4_000, 0x5EED);
        }
        t
    });
    let (r, c) = class.grid_position();
    t[r * 13 + c]
}

/// The weakest preflop strength still inside the strongest `share` of all 1,326 starting combos.
pub fn top_share_strength(share: f64) -> f64 {
    static SORTED: OnceLock<Vec<f64>> = OnceLock::new();
    let v = SORTED.get_or_init(|| {
        let mut v: Vec<f64> = Combo::all().map(|c| preflop_class_strength(HandClass::of(c))).collect();
        v.sort_by(|a, b| b.total_cmp(a));
        v
    });
    let k = ((share.clamp(0.0, 1.0) * v.len() as f64).ceil() as usize).clamp(1, v.len());
    v[k - 1]
}

/// Weighted percentile of each combo's strength within the range: the share of the range's
/// weight that is weaker, plus half of the equal-strength weight. Combos outside the range get 0.
pub fn percentiles(weights: &[f64], strength: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..weights.len()).filter(|&i| weights[i] > 0.0).collect();
    idx.sort_by(|&a, &b| strength[a].total_cmp(&strength[b]));
    let total: f64 = idx.iter().map(|&i| weights[i]).sum();
    let mut out = vec![0.0; weights.len()];
    if total <= 0.0 {
        return out;
    }
    let mut below = 0.0;
    let mut k = 0;
    while k < idx.len() {
        let mut j = k;
        let mut tie = 0.0;
        while j < idx.len() && strength[idx[j]] == strength[idx[k]] {
            tie += weights[idx[j]];
            j += 1;
        }
        for &i in &idx[k..j] {
            out[i] = (below + tie / 2.0) / total;
        }
        below += tie;
        k = j;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo(s: &str) -> Combo {
        let c = Card::parse_many(s).unwrap();
        Combo::new(c[0], c[1]).unwrap()
    }

    #[test]
    fn strength_orders_obvious_hands_and_is_reproducible() {
        let board = Card::parse_many("Ks7d2c").unwrap();
        let set = card_set(&board);
        let sets = strength_vs_random(combo("7h7s"), &board, set, 2000, 1);
        let air = strength_vs_random(combo("4h3h"), &board, set, 2000, 1);
        assert!(sets > 0.9 && air < 0.35, "sets {sets} air {air}");
        assert_eq!(sets, strength_vs_random(combo("7h7s"), &board, set, 2000, 1));
        let aa = preflop_class_strength("AA".parse().unwrap());
        let trash = preflop_class_strength("72o".parse().unwrap());
        assert!((aa - 0.85).abs() < 0.02, "AA vs random is ~85%, got {aa}");
        assert!(trash < 0.37, "72o {trash}");
        assert!(top_share_strength(0.05) > top_share_strength(0.6));
        assert_eq!(top_share_strength(1.0), trash.min(top_share_strength(1.0)));
    }

    #[test]
    fn percentiles_are_weighted_and_handle_ties() {
        let mut w = vec![0.0; 4];
        w[0] = 1.0;
        w[1] = 1.0;
        w[2] = 2.0;
        let s = vec![0.1, 0.1, 0.9, 0.5];
        let p = percentiles(&w, &s);
        assert_eq!(p[0], 0.25); // (0 + 2/2) / 4
        assert_eq!(p[2], 0.75); // (2 + 2/2) / 4
        assert_eq!(p[3], 0.0, "outside the range");
    }
}
