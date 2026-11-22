//! Six-max table positions.
//!
//! Positions are listed in preflop action order: UTG, HJ, CO, BTN, SB, BB. Short-handed
//! tables (3–5 players) drop positions from the front (5-handed starts at HJ, 3-handed is
//! BTN/SB/BB). Heads-up is not supported yet because its blind/action order differs
//! (the button posts the small blind and acts first preflop, last postflop).

use crate::error::{PokerError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Position {
    Utg,
    Hj,
    Co,
    Btn,
    Sb,
    Bb,
}

pub const MIN_PLAYERS: usize = 3;
pub const MAX_PLAYERS: usize = 6;

impl Position {
    /// All six-max positions in preflop action order.
    pub const SIX_MAX: [Position; 6] =
        [Position::Utg, Position::Hj, Position::Co, Position::Btn, Position::Sb, Position::Bb];

    /// Positions occupied at a table of `n` players, in preflop action order.
    pub fn for_players(n: usize) -> Result<&'static [Position]> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&n) {
            return Err(PokerError::InvalidConfig(format!(
                "player count must be between {MIN_PLAYERS} and {MAX_PLAYERS}, got {n}"
            )));
        }
        Ok(&Self::SIX_MAX[MAX_PLAYERS - n..])
    }

    /// Seat indices (into `for_players(n)`) in postflop action order: SB, BB, then the rest
    /// in preflop order, ending with the button.
    pub fn postflop_order(n: usize) -> Vec<usize> {
        let mut v = vec![n - 2, n - 1];
        v.extend(0..n - 2);
        v
    }

    pub const fn short_name(self) -> &'static str {
        match self {
            Position::Utg => "UTG",
            Position::Hj => "HJ",
            Position::Co => "CO",
            Position::Btn => "BTN",
            Position::Sb => "SB",
            Position::Bb => "BB",
        }
    }

    pub const fn long_name(self) -> &'static str {
        match self {
            Position::Utg => "Under the Gun",
            Position::Hj => "Hijack",
            Position::Co => "Cutoff",
            Position::Btn => "Button",
            Position::Sb => "Small Blind",
            Position::Bb => "Big Blind",
        }
    }

    pub const fn is_blind(self) -> bool {
        matches!(self, Position::Sb | Position::Bb)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.short_name())
    }
}

impl FromStr for Position {
    type Err = PokerError;
    fn from_str(s: &str) -> Result<Position> {
        Self::SIX_MAX
            .into_iter()
            .find(|p| p.short_name().eq_ignore_ascii_case(s))
            .ok_or_else(|| PokerError::InvalidConfig(format!("unknown position '{s}'")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_max_orders() {
        let p = Position::for_players(6).unwrap();
        assert_eq!(p, &Position::SIX_MAX);
        let post: Vec<_> = Position::postflop_order(6).into_iter().map(|i| p[i]).collect();
        assert_eq!(post, vec![Position::Sb, Position::Bb, Position::Utg, Position::Hj, Position::Co, Position::Btn]);
    }

    #[test]
    fn short_handed_tables_drop_early_positions() {
        assert_eq!(Position::for_players(3).unwrap(), &[Position::Btn, Position::Sb, Position::Bb]);
        assert_eq!(Position::for_players(5).unwrap()[0], Position::Hj);
        assert!(Position::for_players(2).is_err());
        assert!(Position::for_players(7).is_err());
    }

    #[test]
    fn parse_round_trip() {
        for p in Position::SIX_MAX {
            assert_eq!(p.short_name().parse::<Position>().unwrap(), p);
        }
        assert!("MP".parse::<Position>().is_err());
    }
}
