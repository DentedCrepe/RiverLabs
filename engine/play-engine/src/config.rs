//! Table configuration for a play session.

use poker_core::{Chips, GameConfig};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum PlayError {
    #[error("invalid table configuration: {0}")]
    InvalidConfig(String),
    #[error("illegal action: {0}")]
    IllegalAction(String),
    #[error("{0}")]
    State(String),
    #[error("engine error: {0}")]
    Engine(String),
}

impl From<poker_core::PokerError> for PlayError {
    fn from(e: poker_core::PokerError) -> Self {
        PlayError::Engine(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PlayError>;

/// How strongly a bot follows its strategy. Every level's behaviour is documented in
/// `docs/play.md`; the lower levels add deliberate, named leaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    /// Calls too much, rarely raises, never bluffs.
    Easy,
    /// Mostly follows the model and rules; calls a little too much and bluffs half as often.
    Medium,
    /// Follows the preflop model and the postflop rules exactly.
    Hard,
}

/// One blind level of a sit & go, in tournament chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Level {
    pub small_blind: u64,
    pub big_blind: u64,
    pub ante: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Format {
    /// A fixed table. Stacks carry over between hands; busted bots rebuy to the starting
    /// stack; when you bust you rebuy if `auto_rebuy`, otherwise the session ends.
    Cash { game: GameConfig, auto_rebuy: bool },
    /// A new table every hand: fresh opponents with starting stacks and a random seat for you.
    /// Your stack carries over (and is topped up to the starting stack when below it).
    FastFold { game: GameConfig },
    /// Freezeout: blinds rise every `hands_per_level` hands; the session ends when you bust or win.
    SitAndGo { starting_chips: u64, levels: Vec<Level>, hands_per_level: u32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableConfig {
    pub format: Format,
    /// One entry per opponent (1–5); the table has `opponents.len() + 1` seats.
    pub opponents: Vec<Difficulty>,
    /// Makes the deal and the bots' choices reproducible.
    pub seed: u64,
}

impl TableConfig {
    pub fn seats(&self) -> usize {
        self.opponents.len() + 1
    }

    pub fn validate(&self) -> Result<()> {
        let bad = |m: String| Err(PlayError::InvalidConfig(m));
        if !(1..=5).contains(&self.opponents.len()) {
            return bad(format!("1 to 5 opponents are supported, got {}", self.opponents.len()));
        }
        match &self.format {
            Format::Cash { game, .. } | Format::FastFold { game } => {
                game.validate()?;
                if game.players as usize != self.seats() {
                    return bad(format!(
                        "the game configuration is for {} players but the table has {} seats",
                        game.players,
                        self.seats()
                    ));
                }
                if game.stack.effective_stack < Chips(10_000) {
                    return bad("cash tables need a starting stack of at least 10bb".into());
                }
            }
            Format::SitAndGo { starting_chips, levels, hands_per_level } => {
                if levels.is_empty() || levels.len() > 50 {
                    return bad("give 1 to 50 blind levels".into());
                }
                if *hands_per_level == 0 {
                    return bad("hands_per_level must be positive".into());
                }
                for (i, l) in levels.iter().enumerate() {
                    if l.small_blind == 0 || l.small_blind > l.big_blind || l.ante > l.big_blind {
                        return bad(format!(
                            "level {}: need 0 < small blind <= big blind and ante <= big blind",
                            i + 1
                        ));
                    }
                    if i > 0 && l.big_blind < levels[i - 1].big_blind {
                        return bad(format!("level {}: blinds must not decrease", i + 1));
                    }
                }
                if *starting_chips < 10 * levels[0].big_blind {
                    return bad("the starting stack must be at least 10 big blinds of the first level".into());
                }
                if *starting_chips > 100_000_000 {
                    return bad("the starting stack is too large".into());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation() {
        let cash = |players: u8, opponents: usize| TableConfig {
            format: Format::Cash { game: GameConfig { players, ..GameConfig::default() }, auto_rebuy: true },
            opponents: vec![Difficulty::Hard; opponents],
            seed: 1,
        };
        cash(6, 5).validate().unwrap();
        cash(2, 1).validate().unwrap();
        assert!(cash(6, 4).validate().is_err(), "seat count mismatch");
        assert!(cash(6, 0).validate().is_err());
        let sng = |levels: Vec<Level>, chips: u64| TableConfig {
            format: Format::SitAndGo { starting_chips: chips, levels, hands_per_level: 10 },
            opponents: vec![Difficulty::Easy; 2],
            seed: 1,
        };
        let l = |s, b| Level { small_blind: s, big_blind: b, ante: 0 };
        sng(vec![l(10, 20), l(15, 30)], 1500).validate().unwrap();
        assert!(sng(vec![l(10, 20), l(5, 10)], 1500).validate().is_err());
        assert!(sng(vec![l(10, 20)], 100).validate().is_err());
        assert!(sng(vec![l(30, 20)], 1500).validate().is_err());
        let json = serde_json::to_value(cash(2, 1)).unwrap();
        assert_eq!(json["format"]["type"], "cash");
        assert_eq!(serde_json::from_value::<TableConfig>(json).unwrap(), cash(2, 1));
    }
}
