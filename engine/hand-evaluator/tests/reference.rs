//! Deliberately naive reference evaluator, written independently of the bitmask
//! implementation: sort ranks by (count, rank) and classify. Slow but obviously correct.

use poker_core::Card;

/// Comparable key: (category, tie-break ranks).
pub type Key = (u8, Vec<u8>);

pub fn eval5(cards: &[Card]) -> Key {
    let mut ranks: Vec<u8> = cards.iter().map(|c| c.rank().index()).collect();
    ranks.sort_unstable_by(|a, b| b.cmp(a));
    let flush = cards.iter().all(|c| c.suit() == cards[0].suit());
    let uniq = {
        let mut u = ranks.clone();
        u.dedup();
        u
    };
    let straight_high = if uniq.len() == 5 && ranks[0] - ranks[4] == 4 {
        Some(ranks[0])
    } else if uniq == vec![12, 3, 2, 1, 0] {
        Some(3)
    } else {
        None
    };
    // groups sorted by count desc then rank desc
    let mut groups: Vec<(u8, u8)> =
        uniq.iter().map(|&r| (ranks.iter().filter(|&&x| x == r).count() as u8, r)).collect();
    groups.sort_unstable_by(|a, b| b.cmp(a));
    let counts: Vec<u8> = groups.iter().map(|g| g.0).collect();
    let order: Vec<u8> = groups.iter().map(|g| g.1).collect();
    match (straight_high, flush, counts.as_slice()) {
        (Some(h), true, _) => (8, vec![h]),
        (_, _, [4, 1]) => (7, order),
        (_, _, [3, 2]) => (6, order),
        (_, true, _) => (5, ranks),
        (Some(h), false, _) => (4, vec![h]),
        (_, _, [3, 1, 1]) => (3, order),
        (_, _, [2, 2, 1]) => (2, order),
        (_, _, [2, 1, 1, 1]) => (1, order),
        _ => (0, ranks),
    }
}

pub fn eval7(cards: &[Card]) -> Key {
    let mut best: Option<Key> = None;
    for a in 0..7 {
        for b in a + 1..7 {
            let five: Vec<Card> = (0..7).filter(|&i| i != a && i != b).map(|i| cards[i]).collect();
            let k = eval5(&five);
            if best.as_ref().is_none_or(|bk| k > *bk) {
                best = Some(k);
            }
        }
    }
    best.unwrap()
}
