use poker_core::{Card, CardSet, HoleCards};
use std::fmt;

/// Number of distinct two-card combinations: C(52, 2).
pub const COMBO_COUNT: usize = 1326;

/// A specific two-card holding, indexed `0..1326` by `hi * (hi - 1) / 2 + lo` where
/// `hi > lo` are card indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Combo(u16);

impl Combo {
    pub fn new(a: Card, b: Card) -> Option<Combo> {
        let (hi, lo) = match a.index().cmp(&b.index()) {
            std::cmp::Ordering::Greater => (a.index(), b.index()),
            std::cmp::Ordering::Less => (b.index(), a.index()),
            std::cmp::Ordering::Equal => return None,
        };
        let (hi, lo) = (hi as u16, lo as u16);
        Some(Combo(hi * (hi - 1) / 2 + lo))
    }

    pub fn from_index(i: usize) -> Option<Combo> {
        (i < COMBO_COUNT).then_some(Combo(i as u16))
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// The two cards, higher index first.
    pub fn cards(self) -> (Card, Card) {
        let i = self.0 as u32;
        // invert hi*(hi-1)/2 <= i
        let mut hi = ((1.0 + (1.0 + 8.0 * i as f64).sqrt()) / 2.0) as u32;
        while hi * (hi - 1) / 2 > i {
            hi -= 1;
        }
        while (hi + 1) * hi / 2 <= i {
            hi += 1;
        }
        let lo = i - hi * (hi - 1) / 2;
        (Card::from_index(hi as u8).expect("<52"), Card::from_index(lo as u8).expect("<52"))
    }

    pub fn card_set(self) -> CardSet {
        let (a, b) = self.cards();
        CardSet::from_cards([a, b])
    }

    pub fn hole_cards(self) -> HoleCards {
        let (a, b) = self.cards();
        HoleCards::new(a, b).expect("distinct")
    }

    pub fn all() -> impl Iterator<Item = Combo> {
        (0..COMBO_COUNT as u16).map(Combo)
    }
}

impl From<HoleCards> for Combo {
    fn from(h: HoleCards) -> Combo {
        Combo::new(h.high(), h.low()).expect("hole cards are distinct")
    }
}

impl fmt::Display for Combo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (a, b) = self.cards();
        // Higher rank first for readability.
        if a.rank() >= b.rank() { write!(f, "{a}{b}") } else { write!(f, "{b}{a}") }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_is_a_bijection() {
        let mut seen = vec![false; COMBO_COUNT];
        for a in Card::all() {
            for b in Card::all() {
                if a.index() > b.index() {
                    let c = Combo::new(a, b).unwrap();
                    assert!(!seen[c.index()]);
                    seen[c.index()] = true;
                    assert_eq!(c.cards(), (a, b));
                    assert_eq!(Combo::new(b, a), Some(c));
                }
            }
        }
        assert!(seen.iter().all(|&s| s));
        assert_eq!(Combo::new(Card::from_index(3).unwrap(), Card::from_index(3).unwrap()), None);
    }
}
