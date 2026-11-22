//! Seven-card Texas Hold'em hand evaluator (ADR-0003).
//!
//! [`evaluate`] maps 5, 6 or 7 distinct cards to a [`HandStrength`]: a `u32` whose integer
//! order is exactly poker hand order (greater = better, equal = exact tie, suits never break
//! ties). Layout: bits 20..24 hold the category (0 = high card … 8 = straight flush) and bits
//! 0..20 hold up to five 4-bit rank indices (deuce = 0 … ace = 12), most significant first.
//! For straights only the top card is stored (the wheel's top card is the five).
//!
//! The algorithm is a direct bitmask computation (no lookup tables), so it is deterministic
//! and allocation-free. Correctness is established exhaustively; see `docs/hand-evaluator.md`.

use poker_core::card::{Card, Rank};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandCategory {
    HighCard,
    OnePair,
    TwoPair,
    ThreeOfAKind,
    Straight,
    Flush,
    FullHouse,
    FourOfAKind,
    StraightFlush,
    /// Ace-high straight flush. Reported separately for display; it is simply the best
    /// straight flush in the ordering.
    RoyalFlush,
}

impl HandCategory {
    pub const fn name(self) -> &'static str {
        match self {
            HandCategory::HighCard => "High card",
            HandCategory::OnePair => "One pair",
            HandCategory::TwoPair => "Two pair",
            HandCategory::ThreeOfAKind => "Three of a kind",
            HandCategory::Straight => "Straight",
            HandCategory::Flush => "Flush",
            HandCategory::FullHouse => "Full house",
            HandCategory::FourOfAKind => "Four of a kind",
            HandCategory::StraightFlush => "Straight flush",
            HandCategory::RoyalFlush => "Royal flush",
        }
    }
}

const HIGH_CARD: u32 = 0;
const ONE_PAIR: u32 = 1;
const TWO_PAIR: u32 = 2;
const TRIPS: u32 = 3;
const STRAIGHT: u32 = 4;
const FLUSH: u32 = 5;
const FULL_HOUSE: u32 = 6;
const QUADS: u32 = 7;
const STRAIGHT_FLUSH: u32 = 8;
const CATEGORY_SHIFT: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HandStrength(u32);

impl HandStrength {
    pub const fn value(self) -> u32 {
        self.0
    }

    fn category_code(self) -> u32 {
        self.0 >> CATEGORY_SHIFT
    }

    pub fn category(self) -> HandCategory {
        match self.category_code() {
            HIGH_CARD => HandCategory::HighCard,
            ONE_PAIR => HandCategory::OnePair,
            TWO_PAIR => HandCategory::TwoPair,
            TRIPS => HandCategory::ThreeOfAKind,
            STRAIGHT => HandCategory::Straight,
            FLUSH => HandCategory::Flush,
            FULL_HOUSE => HandCategory::FullHouse,
            QUADS => HandCategory::FourOfAKind,
            _ if self.ranks()[0] == Rank::Ace => HandCategory::RoyalFlush,
            _ => HandCategory::StraightFlush,
        }
    }

    /// The significant ranks in comparison order (e.g. full house: trips rank, pair rank).
    pub fn ranks(self) -> Vec<Rank> {
        let n = match self.category_code() {
            STRAIGHT | STRAIGHT_FLUSH => 1,
            QUADS | FULL_HOUSE => 2,
            TRIPS => 3,
            TWO_PAIR => 3,
            ONE_PAIR => 4,
            _ => 5,
        };
        (0..n)
            .map(|i| {
                let nib = (self.0 >> (16 - 4 * i)) & 0xF;
                Rank::from_index(nib as u8).expect("valid rank nibble")
            })
            .collect()
    }

    /// Human-readable description, e.g. "Full house, kings full of tens".
    pub fn describe(self) -> String {
        let r = self.ranks();
        let name = |r: Rank| rank_name(r, false);
        let plural = |r: Rank| rank_name(r, true);
        match self.category() {
            HandCategory::HighCard => format!("High card, {} high", name(r[0])),
            HandCategory::OnePair => format!("One pair, {}", plural(r[0])),
            HandCategory::TwoPair => format!("Two pair, {} and {}", plural(r[0]), plural(r[1])),
            HandCategory::ThreeOfAKind => format!("Three of a kind, {}", plural(r[0])),
            HandCategory::Straight => format!("Straight, {} high", name(r[0])),
            HandCategory::Flush => format!("Flush, {} high", name(r[0])),
            HandCategory::FullHouse => format!("Full house, {} full of {}", plural(r[0]), plural(r[1])),
            HandCategory::FourOfAKind => format!("Four of a kind, {}", plural(r[0])),
            HandCategory::StraightFlush => format!("Straight flush, {} high", name(r[0])),
            HandCategory::RoyalFlush => "Royal flush".to_string(),
        }
    }
}

impl fmt::Display for HandStrength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.describe())
    }
}

fn rank_name(r: Rank, plural: bool) -> &'static str {
    const S: [&str; 13] =
        ["deuce", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "jack", "queen", "king", "ace"];
    const P: [&str; 13] = [
        "deuces", "threes", "fours", "fives", "sixes", "sevens", "eights", "nines", "tens", "jacks", "queens", "kings",
        "aces",
    ];
    if plural { P[r.index() as usize] } else { S[r.index() as usize] }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalError {
    /// Hands must contain 5 to 7 cards.
    WrongCardCount(usize),
    DuplicateCard(Card),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::WrongCardCount(n) => write!(f, "a hand must contain 5 to 7 cards, got {n}"),
            EvalError::DuplicateCard(c) => write!(f, "duplicate card {c}"),
        }
    }
}
impl std::error::Error for EvalError {}

/// Evaluates 5 to 7 distinct cards. Rejects other sizes and duplicates.
pub fn evaluate(cards: &[Card]) -> Result<HandStrength, EvalError> {
    if !(5..=7).contains(&cards.len()) {
        return Err(EvalError::WrongCardCount(cards.len()));
    }
    let mut seen = 0u64;
    for &c in cards {
        let bit = 1u64 << c.index();
        if seen & bit != 0 {
            return Err(EvalError::DuplicateCard(c));
        }
        seen |= bit;
    }
    Ok(evaluate_mask(seen))
}

/// Evaluates a 52-bit card mask (bit `i` = card index `i`) holding 5 to 7 cards.
/// The caller guarantees the count; this is the hot path used by equity enumeration.
#[inline]
pub fn evaluate_mask(mask: u64) -> HandStrength {
    debug_assert!((5..=7).contains(&mask.count_ones()), "evaluate_mask needs 5-7 cards");
    // Per-suit 13-bit rank masks.
    let mut suits = [0u16; 4];
    let mut counts = [0u8; 13];
    let mut m = mask;
    while m != 0 {
        let i = m.trailing_zeros();
        m &= m - 1;
        let (rank, suit) = (i >> 2, i & 3);
        suits[suit as usize] |= 1 << rank;
        counts[rank as usize] += 1;
    }

    // Flush / straight flush. With at most 7 cards only one suit can hold five or more.
    for &sm in &suits {
        if sm.count_ones() >= 5 {
            if let Some(high) = straight_high(sm) {
                return make(STRAIGHT_FLUSH, &[high]);
            }
            return make(FLUSH, &top_ranks(sm, 5));
        }
    }

    let mut quads = 0u16;
    let mut trips = 0u16;
    let mut pairs = 0u16;
    let mut singles = 0u16;
    for (r, &c) in counts.iter().enumerate() {
        let b = 1u16 << r;
        match c {
            4 => quads |= b,
            3 => trips |= b,
            2 => pairs |= b,
            1 => singles |= b,
            _ => {}
        }
    }
    let all = quads | trips | pairs | singles;

    if quads != 0 {
        let q = high_bit(quads);
        let kicker = high_bit(all & !(1 << q));
        return make(QUADS, &[q, kicker]);
    }
    if trips != 0 {
        let t = high_bit(trips);
        let rest = (trips & !(1 << t)) | pairs;
        if rest != 0 {
            return make(FULL_HOUSE, &[t, high_bit(rest)]);
        }
    }
    if let Some(high) = straight_high(all) {
        return make(STRAIGHT, &[high]);
    }
    if trips != 0 {
        let t = high_bit(trips);
        let k = top_ranks(all & !(1 << t), 2);
        return make(TRIPS, &[t, k[0], k[1]]);
    }
    if pairs.count_ones() >= 2 {
        let p = top_ranks(pairs, 2);
        let kicker = high_bit(all & !(1 << p[0]) & !(1 << p[1]));
        return make(TWO_PAIR, &[p[0], p[1], kicker]);
    }
    if pairs != 0 {
        let p = high_bit(pairs);
        let k = top_ranks(all & !(1 << p), 3);
        return make(ONE_PAIR, &[p, k[0], k[1], k[2]]);
    }
    make(HIGH_CARD, &top_ranks(all, 5))
}

#[inline]
fn high_bit(m: u16) -> u32 {
    debug_assert!(m != 0);
    15 - m.leading_zeros()
}

#[inline]
fn top_ranks(mut m: u16, n: usize) -> [u32; 5] {
    let mut out = [0u32; 5];
    for slot in out.iter_mut().take(n) {
        let h = high_bit(m);
        *slot = h;
        m &= !(1 << h);
    }
    out
}

/// Highest straight in a 13-bit rank mask, returning the rank index of its top card.
/// The ace also plays low (A-2-3-4-5, top card five).
#[inline]
fn straight_high(m: u16) -> Option<u32> {
    // ext bit 0 = ace-low, ext bit r+1 = rank r.
    let ext = ((m as u32) << 1) | ((m as u32 >> 12) & 1);
    let s = ext & (ext >> 1) & (ext >> 2) & (ext >> 3) & (ext >> 4);
    if s == 0 { None } else { Some((31 - s.leading_zeros()) + 3) }
}

#[inline]
fn make(cat: u32, ranks: &[u32]) -> HandStrength {
    let mut v = cat << CATEGORY_SHIFT;
    for (i, &r) in ranks.iter().take(5).enumerate() {
        v |= r << (16 - 4 * i as u32);
    }
    // Unused nibble positions for categories with fewer significant ranks are zero, so they
    // never influence comparison between hands of the same category.
    let used = match cat {
        STRAIGHT | STRAIGHT_FLUSH => 1,
        QUADS | FULL_HOUSE => 2,
        TRIPS | TWO_PAIR => 3,
        ONE_PAIR => 4,
        _ => 5,
    };
    let keep_bits = 4 * used as u32;
    let mask = ((1u32 << keep_bits) - 1) << (20 - keep_bits);
    HandStrength((v & (0xF << CATEGORY_SHIFT)) | (v & mask))
}

/// The best five-card combination among 5–7 cards (for display). Ties between equal-strength
/// five-card subsets are broken by lowest card indices, so the output is deterministic.
pub fn best_five(cards: &[Card]) -> Result<([Card; 5], HandStrength), EvalError> {
    let strength = evaluate(cards)?;
    let n = cards.len();
    let mut best: Option<[Card; 5]> = None;
    for a in 0..n {
        for b in a + 1..n {
            for c in b + 1..n {
                for d in c + 1..n {
                    for e in d + 1..n {
                        let five = [cards[a], cards[b], cards[c], cards[d], cards[e]];
                        let s = evaluate(&five).expect("distinct");
                        if s == strength {
                            let mut sorted = five;
                            sorted.sort_by(|x, y| y.cmp(x));
                            if best.is_none_or(|bst| sorted < bst) {
                                best = Some(sorted);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok((best.expect("some 5-card subset attains the 7-card strength"), strength))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str) -> HandStrength {
        evaluate(&Card::parse_many(s).unwrap()).unwrap()
    }

    #[test]
    fn categories() {
        let cases = [
            ("AsKsQsJsTs2c3d", HandCategory::RoyalFlush),
            ("9s8s7s6s5s2c3d", HandCategory::StraightFlush),
            ("As2s3s4s5sKdQd", HandCategory::StraightFlush),
            ("AsAdAhAc2c3d4h", HandCategory::FourOfAKind),
            ("KsKdKhQcQd2c3d", HandCategory::FullHouse),
            ("As9s7s5s2sKdQd", HandCategory::Flush),
            ("Ts9d8h7c6s2c2d", HandCategory::Straight),
            ("Ad2c3h4s5dKcQd", HandCategory::Straight),
            ("7s7d7h2c3d9hKs", HandCategory::ThreeOfAKind),
            ("7s7d2h2c3d9hKs", HandCategory::TwoPair),
            ("7s7d2h4c3d9hKs", HandCategory::OnePair),
            ("As7d2h4c3d9hKs", HandCategory::HighCard),
        ];
        for (h, cat) in cases {
            assert_eq!(ev(h).category(), cat, "{h}");
        }
    }

    #[test]
    fn wheel_is_the_lowest_straight() {
        assert!(ev("Ad2c3h4s5d") < ev("2c3h4s5d6h"));
        assert_eq!(ev("Ad2c3h4s5d").ranks(), vec![Rank::Five]);
        assert!(ev("As2s3s4s5s") < ev("2h3h4h5h6h"));
        // Wrap-around is not a straight.
        assert_eq!(ev("QsKdAc2h3d").category(), HandCategory::HighCard);
    }

    #[test]
    fn kickers_decide() {
        assert!(ev("AsAdKc7h3d") > ev("AhAcQs7d3c"));
        assert!(ev("AsAdKcQh3d") > ev("AhAcKsJd3c"));
        assert!(ev("AsAdKcQh4d") > ev("AhAcKsQd3c"));
        // Only five cards play: the sixth/seventh card cannot break a tie.
        assert_eq!(ev("AsAdKcQhJd3c2s"), ev("AhAcKsQdJc4d3h"));
        // Two pair kicker can come from a third pair.
        assert_eq!(ev("KsKdQhQc2s2dAh").ranks(), vec![Rank::King, Rank::Queen, Rank::Ace]);
        assert_eq!(ev("KsKdQhQc5s5d2h").ranks(), vec![Rank::King, Rank::Queen, Rank::Five]);
    }

    #[test]
    fn full_house_comparisons() {
        assert!(ev("KsKdKh2c2d") > ev("QsQdQhAcAd"));
        assert!(ev("KsKdKhAcAd") > ev("KsKdKhQcQd"));
        // Two sets: higher trips with lower trips as the pair.
        assert_eq!(ev("9s9d9h5c5d5h2s").ranks(), vec![Rank::Nine, Rank::Five]);
    }

    #[test]
    fn flush_comparisons_use_all_five_cards() {
        assert!(ev("AsJs9s7s3s") > ev("AhJh9h7h2h"));
        assert!(ev("AsKs3s4s6s2d") > ev("AhQhJhTh8h"));
        // Six suited cards: best five used.
        assert_eq!(ev("As9s7s5s3s2sKd").ranks(), ev("As9s7s5s3s").ranks());
    }

    #[test]
    fn board_plays_ties() {
        // Board is a broadway straight; both players play the board.
        let board = "AsKdQhJcTd";
        assert_eq!(ev(&format!("{board}2c3c")), ev(&format!("{board}4h5h")));
    }

    #[test]
    fn suits_never_break_ties() {
        assert_eq!(ev("AsKdQh9c7d"), ev("AcKhQs9d7s"));
    }

    #[test]
    fn errors() {
        assert_eq!(evaluate(&Card::parse_many("AsKd").unwrap()), Err(EvalError::WrongCardCount(2)));
        assert!(matches!(evaluate(&Card::parse_many("AsAsKdQh2c").unwrap()), Err(EvalError::DuplicateCard(_))));
    }

    #[test]
    fn describe_and_best_five() {
        assert_eq!(ev("KsKdKhTcTd2c3d").describe(), "Full house, kings full of tens");
        assert_eq!(ev("AsKsQsJsTs").describe(), "Royal flush");
        let (five, _) = best_five(&Card::parse_many("AsKsQsJsTs2c3d").unwrap()).unwrap();
        assert!(five.iter().all(|c| c.suit() == poker_core::Suit::Spades));
    }
}
