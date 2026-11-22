//! A set of cards as a 52-bit mask. Used for dead-card / card-removal bookkeeping.

use crate::card::Card;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CardSet(u64);

impl CardSet {
    pub const EMPTY: CardSet = CardSet(0);
    pub const FULL: CardSet = CardSet((1u64 << 52) - 1);

    pub fn from_cards<I: IntoIterator<Item = Card>>(cards: I) -> CardSet {
        let mut s = CardSet::EMPTY;
        for c in cards {
            s.insert(c);
        }
        s
    }

    /// Builds a set, returning the first duplicated card if any.
    pub fn from_distinct(cards: &[Card]) -> Result<CardSet, Card> {
        let mut s = CardSet::EMPTY;
        for &c in cards {
            if !s.insert(c) {
                return Err(c);
            }
        }
        Ok(s)
    }

    #[inline]
    pub const fn bits(self) -> u64 {
        self.0
    }
    #[inline]
    pub fn contains(self, c: Card) -> bool {
        self.0 & (1u64 << c.index()) != 0
    }
    /// Inserts `c`; returns `false` if it was already present.
    #[inline]
    pub fn insert(&mut self, c: Card) -> bool {
        let bit = 1u64 << c.index();
        let fresh = self.0 & bit == 0;
        self.0 |= bit;
        fresh
    }
    #[inline]
    pub fn remove(&mut self, c: Card) -> bool {
        let bit = 1u64 << c.index();
        let had = self.0 & bit != 0;
        self.0 &= !bit;
        had
    }
    #[inline]
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
    #[inline]
    pub const fn union(self, o: CardSet) -> CardSet {
        CardSet(self.0 | o.0)
    }
    #[inline]
    pub const fn intersects(self, o: CardSet) -> bool {
        self.0 & o.0 != 0
    }
    #[inline]
    pub const fn complement(self) -> CardSet {
        CardSet(!self.0 & Self::FULL.0)
    }

    pub fn iter(self) -> impl Iterator<Item = Card> {
        let mut bits = self.0;
        std::iter::from_fn(move || {
            if bits == 0 {
                None
            } else {
                let i = bits.trailing_zeros() as u8;
                bits &= bits - 1;
                Some(Card::from_index(i).expect("bit index < 52"))
            }
        })
    }
}

impl fmt::Debug for CardSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter().map(|c| c.to_string())).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn insert_remove_len() {
        let a = Card::from_str("As").unwrap();
        let mut s = CardSet::EMPTY;
        assert!(s.insert(a));
        assert!(!s.insert(a));
        assert_eq!(s.len(), 1);
        assert!(s.contains(a));
        assert!(s.remove(a));
        assert!(s.is_empty());
        assert_eq!(CardSet::FULL.len(), 52);
        assert_eq!(CardSet::FULL.iter().count(), 52);
        assert!(CardSet::FULL.complement().is_empty());
    }

    #[test]
    fn detects_duplicates() {
        let c = Card::parse_many("AsKsAs").unwrap();
        assert_eq!(CardSet::from_distinct(&c), Err(c[0]));
    }
}
