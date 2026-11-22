//! A deck of remaining cards with deterministic, caller-seeded shuffling.

use crate::card::Card;
use crate::cardset::CardSet;
use crate::error::{PokerError, Result};
use rand::Rng;
use rand::seq::SliceRandom;

#[derive(Debug, Clone)]
pub struct Deck {
    cards: Vec<Card>,
}

impl Default for Deck {
    fn default() -> Self {
        Self::new()
    }
}

impl Deck {
    /// A full, ordered 52-card deck.
    pub fn new() -> Deck {
        Deck { cards: Card::all().collect() }
    }

    /// A deck without the given dead cards.
    pub fn without(dead: CardSet) -> Deck {
        Deck { cards: Card::all().filter(|c| !dead.contains(*c)).collect() }
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
    pub fn contains(&self, c: Card) -> bool {
        self.cards.contains(&c)
    }
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    /// Fisher–Yates shuffle driven by the caller's RNG; reproducible for a seeded RNG.
    pub fn shuffle<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        self.cards.shuffle(rng);
    }

    /// Deals from the top (end) of the deck.
    pub fn deal(&mut self) -> Result<Card> {
        self.cards.pop().ok_or(PokerError::DeckExhausted)
    }

    pub fn deal_n(&mut self, n: usize) -> Result<Vec<Card>> {
        if n > self.cards.len() {
            return Err(PokerError::DeckExhausted);
        }
        Ok(self.cards.split_off(self.cards.len() - n).into_iter().rev().collect())
    }

    /// Removes a specific card (e.g. a known board card).
    pub fn remove(&mut self, c: Card) -> Result<()> {
        let pos = self.cards.iter().position(|&x| x == c).ok_or_else(|| PokerError::CardNotInDeck(c.to_string()))?;
        self.cards.remove(pos);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::str::FromStr;

    #[test]
    fn full_deck_has_52_distinct_cards() {
        let d = Deck::new();
        assert_eq!(d.len(), 52);
        assert_eq!(CardSet::from_cards(d.cards().iter().copied()).len(), 52);
    }

    #[test]
    fn dealing_never_repeats_and_exhausts() {
        let mut d = Deck::new();
        d.shuffle(&mut ChaCha8Rng::seed_from_u64(7));
        let mut seen = CardSet::EMPTY;
        for _ in 0..52 {
            assert!(seen.insert(d.deal().unwrap()));
        }
        assert_eq!(d.deal(), Err(PokerError::DeckExhausted));
        assert!(Deck::new().deal_n(53).is_err());
    }

    #[test]
    fn shuffle_is_reproducible_for_a_seed() {
        let mut a = Deck::new();
        let mut b = Deck::new();
        a.shuffle(&mut ChaCha8Rng::seed_from_u64(42));
        b.shuffle(&mut ChaCha8Rng::seed_from_u64(42));
        assert_eq!(a.cards(), b.cards());
    }

    #[test]
    fn remove_and_without() {
        let mut d = Deck::new();
        let a = Card::from_str("Ah").unwrap();
        d.remove(a).unwrap();
        assert!(d.remove(a).is_err());
        assert_eq!(Deck::without(CardSet::from_cards([a])).len(), 51);
    }
}
