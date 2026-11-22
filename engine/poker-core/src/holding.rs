//! Hole cards, community cards (board) and streets.

use crate::card::Card;
use crate::cardset::CardSet;
use crate::error::{PokerError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
}

impl Street {
    /// Number of community cards visible on this street.
    pub const fn board_len(self) -> usize {
        match self {
            Street::Preflop => 0,
            Street::Flop => 3,
            Street::Turn => 4,
            Street::River => 5,
        }
    }
    pub const fn next(self) -> Option<Street> {
        match self {
            Street::Preflop => Some(Street::Flop),
            Street::Flop => Some(Street::Turn),
            Street::Turn => Some(Street::River),
            Street::River => None,
        }
    }
    pub fn from_board_len(n: usize) -> Result<Street> {
        match n {
            0 => Ok(Street::Preflop),
            3 => Ok(Street::Flop),
            4 => Ok(Street::Turn),
            5 => Ok(Street::River),
            _ => Err(PokerError::InvalidBoard(format!("a board has 0, 3, 4 or 5 cards, got {n}"))),
        }
    }
}

/// Two distinct private cards, stored higher card first (by index) for a canonical form.
/// Serialized as a 4-character string such as `"AsKd"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct HoleCards([Card; 2]);

impl TryFrom<String> for HoleCards {
    type Error = PokerError;
    fn try_from(s: String) -> Result<HoleCards> {
        s.parse()
    }
}
impl From<HoleCards> for String {
    fn from(h: HoleCards) -> String {
        h.to_string()
    }
}

impl HoleCards {
    pub fn new(a: Card, b: Card) -> Result<HoleCards> {
        if a == b {
            return Err(PokerError::InvalidHoleCards(format!("duplicate card {a}")));
        }
        Ok(if a > b { HoleCards([a, b]) } else { HoleCards([b, a]) })
    }
    pub fn cards(&self) -> [Card; 2] {
        self.0
    }
    pub fn high(&self) -> Card {
        self.0[0]
    }
    pub fn low(&self) -> Card {
        self.0[1]
    }
    pub fn card_set(&self) -> CardSet {
        CardSet::from_cards(self.0)
    }
}

impl FromStr for HoleCards {
    type Err = PokerError;
    fn from_str(s: &str) -> Result<HoleCards> {
        let c = Card::parse_many(s)?;
        if c.len() != 2 {
            return Err(PokerError::InvalidHoleCards(format!("expected 2 cards, got '{s}'")));
        }
        HoleCards::new(c[0], c[1])
    }
}

impl fmt::Display for HoleCards {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.0[0], self.0[1])
    }
}

/// Community cards: 0, 3, 4 or 5 distinct cards in dealing order.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(try_from = "Vec<Card>", into = "Vec<Card>")]
pub struct Board {
    cards: Vec<Card>,
}

impl Board {
    pub fn empty() -> Board {
        Board { cards: Vec::new() }
    }
    pub fn new(cards: Vec<Card>) -> Result<Board> {
        Street::from_board_len(cards.len())?;
        if let Err(dup) = CardSet::from_distinct(&cards) {
            return Err(PokerError::DuplicateCard(dup.to_string()));
        }
        Ok(Board { cards })
    }
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }
    pub fn street(&self) -> Street {
        Street::from_board_len(self.cards.len()).expect("validated on construction")
    }
    pub fn card_set(&self) -> CardSet {
        CardSet::from_cards(self.cards.iter().copied())
    }
    /// Appends the cards for the next street (3 for the flop, 1 for turn and river).
    pub fn deal_next(&mut self, cards: &[Card]) -> Result<Street> {
        let next = self.street().next().ok_or_else(|| PokerError::InvalidBoard("board is already complete".into()))?;
        let need = next.board_len() - self.cards.len();
        if cards.len() != need {
            return Err(PokerError::InvalidBoard(format!("{next:?} requires {need} card(s), got {}", cards.len())));
        }
        let mut all = self.cards.clone();
        all.extend_from_slice(cards);
        *self = Board::new(all)?;
        Ok(next)
    }
}

impl TryFrom<Vec<Card>> for Board {
    type Error = PokerError;
    fn try_from(v: Vec<Card>) -> Result<Board> {
        Board::new(v)
    }
}
impl From<Board> for Vec<Card> {
    fn from(b: Board) -> Vec<Card> {
        b.cards
    }
}

impl FromStr for Board {
    type Err = PokerError;
    fn from_str(s: &str) -> Result<Board> {
        Board::new(Card::parse_many(s)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hole_cards_are_canonical_and_distinct() {
        let a: HoleCards = "KdAs".parse().unwrap();
        let b: HoleCards = "AsKd".parse().unwrap();
        assert_eq!(a, b);
        assert_eq!(a.to_string(), "AsKd");
        assert!("AsAs".parse::<HoleCards>().is_err());
        assert!("AsKdQc".parse::<HoleCards>().is_err());
    }

    #[test]
    fn board_validation() {
        assert_eq!("".parse::<Board>().unwrap().street(), Street::Preflop);
        assert_eq!("AsKdQc".parse::<Board>().unwrap().street(), Street::Flop);
        assert!("AsKd".parse::<Board>().is_err());
        assert!("AsKdQcJhTs9s".parse::<Board>().is_err());
        assert!(matches!("AsKdAs".parse::<Board>(), Err(PokerError::DuplicateCard(_))));
    }

    #[test]
    fn deal_next_enforces_street_sizes() {
        let mut b = Board::empty();
        assert!(b.deal_next(&Card::parse_many("As").unwrap()).is_err());
        assert_eq!(b.deal_next(&Card::parse_many("AsKdQc").unwrap()).unwrap(), Street::Flop);
        assert!(b.deal_next(&Card::parse_many("As").unwrap()).is_err(), "duplicate");
        assert_eq!(b.deal_next(&Card::parse_many("2c").unwrap()).unwrap(), Street::Turn);
        assert_eq!(b.deal_next(&Card::parse_many("3c").unwrap()).unwrap(), Street::River);
        assert!(b.deal_next(&Card::parse_many("4c").unwrap()).is_err());
    }

    #[test]
    fn board_serde_validates() {
        let ok: Board = serde_json::from_str(r#"["As","Kd","Qc"]"#).unwrap();
        assert_eq!(ok.cards().len(), 3);
        assert!(serde_json::from_str::<Board>(r#"["As","Kd"]"#).is_err());
    }
}
