//! Exhaustive and differential validation of the evaluator.

mod reference;

use hand_evaluator::{HandCategory, HandStrength, evaluate, evaluate_mask};
use poker_core::Card;
use proptest::prelude::*;
use std::collections::{HashMap, HashSet};

fn cat_index(c: HandCategory) -> usize {
    match c {
        HandCategory::RoyalFlush => 9,
        other => other as usize,
    }
}

/// Every 5-card hand: textbook category frequencies and exactly 7,462 equivalence classes.
#[test]
fn all_five_card_hands() {
    let mut counts = [0u64; 10];
    let mut distinct: HashMap<usize, HashSet<u32>> = HashMap::new();
    let mut all = HashSet::new();
    for a in 0..52u64 {
        for b in a + 1..52 {
            for c in b + 1..52 {
                for d in c + 1..52 {
                    for e in d + 1..52 {
                        let s = evaluate_mask(1 << a | 1 << b | 1 << c | 1 << d | 1 << e);
                        let k = cat_index(s.category());
                        counts[k] += 1;
                        distinct.entry(k).or_default().insert(s.value());
                        all.insert(s.value());
                    }
                }
            }
        }
    }
    // high, pair, two pair, trips, straight, flush, full house, quads, SF (excl. royal), royal
    assert_eq!(counts, [1_302_540, 1_098_240, 123_552, 54_912, 10_200, 5_108, 3_744, 624, 36, 4]);
    let classes: Vec<usize> = (0..10).map(|k| distinct.get(&k).map_or(0, |s| s.len())).collect();
    assert_eq!(classes, vec![1277, 2860, 858, 858, 10, 1277, 156, 156, 9, 1]);
    assert_eq!(all.len(), 7462);
}

/// Every 7-card hand (133,784,560). Takes a few seconds in release mode; run with
/// `cargo test --release -p hand-evaluator -- --ignored` (scripts/validate-engine.sh).
#[test]
#[ignore = "exhaustive 133M-hand enumeration; run via scripts/validate-engine.sh"]
fn all_seven_card_hands() {
    let mut counts = [0u64; 10];
    let mut distinct = HashSet::new();
    let mut total = 0u64;
    for a in 0..52u64 {
        for b in a + 1..52 {
            for c in b + 1..52 {
                for d in c + 1..52 {
                    let m4 = 1 << a | 1 << b | 1 << c | 1 << d;
                    for e in d + 1..52 {
                        for f in e + 1..52 {
                            for g in f + 1..52 {
                                let s = evaluate_mask(m4 | 1 << e | 1 << f | 1 << g);
                                counts[cat_index(s.category())] += 1;
                                distinct.insert(s.value());
                                total += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(total, 133_784_560);
    assert_eq!(
        counts,
        [23_294_460, 58_627_800, 31_433_400, 6_461_620, 6_180_020, 4_047_644, 3_473_184, 224_848, 37_260, 4_324]
    );
    // Only 4,824 of the 7,462 five-card classes can be the best hand of seven cards.
    assert_eq!(distinct.len(), 4824);
}

fn cards_from(idx: &[u8]) -> Vec<Card> {
    idx.iter().map(|&i| Card::from_index(i).unwrap()).collect()
}

fn seven_distinct() -> impl Strategy<Value = Vec<u8>> {
    proptest::sample::subsequence((0u8..52).collect::<Vec<_>>(), 7).prop_shuffle()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20_000))]

    /// The 7-card value equals the best of its 21 five-card subsets.
    #[test]
    fn seven_equals_best_five_subset(idx in seven_distinct()) {
        let cards = cards_from(&idx);
        let s7 = evaluate(&cards).unwrap();
        let mut best = None::<HandStrength>;
        for skip_a in 0..7 {
            for skip_b in skip_a + 1..7 {
                let five: Vec<Card> = (0..7).filter(|&i| i != skip_a && i != skip_b).map(|i| cards[i]).collect();
                let s = evaluate(&five).unwrap();
                best = Some(best.map_or(s, |b| b.max(s)));
            }
        }
        prop_assert_eq!(Some(s7), best);
    }

    /// Order of input cards never matters.
    #[test]
    fn permutation_invariant(idx in seven_distinct(), seed in any::<u64>()) {
        let mut cards = cards_from(&idx);
        let a = evaluate(&cards).unwrap();
        let k = (seed % 7) as usize;
        cards.rotate_left(k);
        cards.reverse();
        prop_assert_eq!(evaluate(&cards).unwrap(), a);
    }

    /// Pairwise ordering agrees with an independently written reference evaluator.
    #[test]
    fn agrees_with_reference(a in seven_distinct(), b in seven_distinct()) {
        let (ca, cb) = (cards_from(&a), cards_from(&b));
        let ours = evaluate(&ca).unwrap().cmp(&evaluate(&cb).unwrap());
        let theirs = reference::eval7(&ca).cmp(&reference::eval7(&cb));
        prop_assert_eq!(ours, theirs);
    }
}
