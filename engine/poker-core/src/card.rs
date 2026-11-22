//! Card, rank and suit primitives.
//!
//! A [`Card`] is a `u8` index `rank * 4 + suit` in `0..52`. Ranks are ordered deuce (0) to
//! ace (12); suits are clubs, diamonds, hearts, spades (0..4). Suits carry no strength.

use crate::error::{PokerError, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Rank {
    Two = 0,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    pub const ALL: [Rank; 13] = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];

    #[inline]
    pub const fn index(self) -> u8 {
        self as u8
    }

    pub fn from_index(i: u8) -> Option<Rank> {
        Rank::ALL.get(i as usize).copied()
    }

    pub const fn to_char(self) -> char {
        match self {
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
            Rank::Ace => 'A',
        }
    }

    pub fn from_char(c: char) -> Result<Rank> {
        Ok(match c.to_ascii_uppercase() {
            '2' => Rank::Two,
            '3' => Rank::Three,
            '4' => Rank::Four,
            '5' => Rank::Five,
            '6' => Rank::Six,
            '7' => Rank::Seven,
            '8' => Rank::Eight,
            '9' => Rank::Nine,
            'T' => Rank::Ten,
            'J' => Rank::Jack,
            'Q' => Rank::Queen,
            'K' => Rank::King,
            'A' => Rank::Ace,
            _ => return Err(PokerError::InvalidRank(c)),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Suit {
    Clubs = 0,
    Diamonds,
    Hearts,
    Spades,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

    #[inline]
    pub const fn index(self) -> u8 {
        self as u8
    }

    pub fn from_index(i: u8) -> Option<Suit> {
        Suit::ALL.get(i as usize).copied()
    }

    pub const fn to_char(self) -> char {
        match self {
            Suit::Clubs => 'c',
            Suit::Diamonds => 'd',
            Suit::Hearts => 'h',
            Suit::Spades => 's',
        }
    }

    pub fn from_char(c: char) -> Result<Suit> {
        Ok(match c.to_ascii_lowercase() {
            'c' => Suit::Clubs,
            'd' => Suit::Diamonds,
            'h' => Suit::Hearts,
            's' => Suit::Spades,
            _ => return Err(PokerError::InvalidSuit(c)),
        })
    }
}

/// A playing card, stored as `rank * 4 + suit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Card(u8);

impl Card {
    pub const fn new(rank: Rank, suit: Suit) -> Card {
        Card(rank as u8 * 4 + suit as u8)
    }

    pub fn from_index(i: u8) -> Result<Card> {
        if i < 52 { Ok(Card(i)) } else { Err(PokerError::InvalidCard(format!("index {i}"))) }
    }

    #[inline]
    pub const fn index(self) -> u8 {
        self.0
    }

    #[inline]
    pub fn rank(self) -> Rank {
        Rank::ALL[(self.0 >> 2) as usize]
    }

    #[inline]
    pub fn suit(self) -> Suit {
        Suit::ALL[(self.0 & 3) as usize]
    }

    /// All 52 cards in index order.
    pub fn all() -> impl Iterator<Item = Card> {
        (0..52u8).map(Card)
    }

    /// Parse a whitespace/comma-free concatenation such as `"AsKd7h"`.
    pub fn parse_many(s: &str) -> Result<Vec<Card>> {
        let chars: Vec<char> = s.chars().filter(|c| !c.is_whitespace() && *c != ',').collect();
        if chars.len() % 2 != 0 {
            return Err(PokerError::InvalidCard(s.to_string()));
        }
        chars.chunks(2).map(|p| Card::from_str(&p.iter().collect::<String>())).collect()
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank().to_char(), self.suit().to_char())
    }
}

impl FromStr for Card {
    type Err = PokerError;
    fn from_str(s: &str) -> Result<Card> {
        let mut it = s.chars();
        match (it.next(), it.next(), it.next()) {
            (Some(r), Some(su), None) => {
                let rank = Rank::from_char(r).map_err(|_| PokerError::InvalidCard(s.into()))?;
                let suit = Suit::from_char(su).map_err(|_| PokerError::InvalidCard(s.into()))?;
                Ok(Card::new(rank, suit))
            }
            _ => Err(PokerError::InvalidCard(s.to_string())),
        }
    }
}

impl Serialize for Card {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Card {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Card, D::Error> {
        let s = String::deserialize(d)?;
        Card::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_all_52_cards() {
        let mut seen = std::collections::HashSet::new();
        for c in Card::all() {
            let s = c.to_string();
            assert_eq!(Card::from_str(&s).unwrap(), c);
            assert_eq!(Card::new(c.rank(), c.suit()), c);
            assert!(seen.insert(s));
        }
        assert_eq!(seen.len(), 52);
    }

    #[test]
    fn parses_case_insensitively_and_rejects_garbage() {
        assert_eq!(Card::from_str("aS").unwrap(), Card::new(Rank::Ace, Suit::Spades));
        assert_eq!(Card::from_str("tc").unwrap(), Card::new(Rank::Ten, Suit::Clubs));
        for bad in ["", "A", "1s", "Ax", "10s", "Asd", "  "] {
            assert!(Card::from_str(bad).is_err(), "{bad:?} should fail");
        }
        assert!(Card::from_index(52).is_err());
    }

    #[test]
    fn rank_order_is_deuce_to_ace() {
        assert!(Rank::Ace > Rank::King && Rank::Two < Rank::Three);
        assert_eq!(Rank::ALL.len(), 13);
    }

    #[test]
    fn parse_many_handles_separators() {
        let v = Card::parse_many("As Kd,7h").unwrap();
        assert_eq!(v.len(), 3);
        assert!(Card::parse_many("AsK").is_err());
    }
}
