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

/// A documented river leak a bot can be given, to practise exploiting it (docs/play.md). It
/// changes only that bot's river rules; the range tracking follows it exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Leak {
    #[default]
    None,
    /// River bets: three times the balanced number of bluffs.
    OverbluffsRiver,
    /// River bets: no bluffs at all.
    NeverBluffsRiver,
    /// Facing a river bet: continues with half the minimum defence share.
    OverfoldsRiver,
    /// Facing a river bet: continues with 1.6 times the minimum defence share.
    CallsTooMuchRiver,
}

impl Leak {
    pub fn label(self) -> &'static str {
        match self {
            Leak::None => "no leak",
            Leak::OverbluffsRiver => "over-bluffs the river",
            Leak::NeverBluffsRiver => "never bluffs the river",
            Leak::OverfoldsRiver => "folds too much to river bets",
            Leak::CallsTooMuchRiver => "calls too much on the river",
        }
    }
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
    SitAndGo {
        starting_chips: u64,
        levels: Vec<Level>,
        hands_per_level: u32,
        /// Percent of the prize pool for 1st, 2nd, ... (whole numbers adding up to 100). Empty:
        /// the default for the table size (`default_payouts`). Used for ICM grading.
        #[serde(default)]
        payouts: Vec<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableConfig {
    pub format: Format,
    /// One entry per opponent (1–5); the table has `opponents.len() + 1` seats.
    pub opponents: Vec<Difficulty>,
    /// Makes the deal and the bots' choices reproducible.
    pub seed: u64,
    /// Optional river leak per opponent (empty: none; otherwise one per opponent).
    #[serde(default)]
    pub leaks: Vec<Leak>,
}

/// Prize-pool percentages used when a sit & go gives none: winner takes all with up to three
/// players, 65/35 with four to six. A common structure for small sit & goes, not a rule.
pub fn default_payouts(players: usize) -> Vec<u32> {
    if players <= 3 { vec![100] } else { vec![65, 35] }
}

impl TableConfig {
    pub fn seats(&self) -> usize {
        self.opponents.len() + 1
    }

    /// Opponent `i`'s leak (0-based among the opponents).
    pub fn leak(&self, i: usize) -> Leak {
        self.leaks.get(i).copied().unwrap_or_default()
    }

    /// The sit & go payouts in effect (`None` for cash tables).
    pub fn payouts(&self) -> Option<Vec<u32>> {
        match &self.format {
            Format::SitAndGo { payouts, .. } if !payouts.is_empty() => Some(payouts.clone()),
            Format::SitAndGo { .. } => Some(default_payouts(self.seats())),
            _ => None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        let bad = |m: String| Err(PlayError::InvalidConfig(m));
        if !(1..=5).contains(&self.opponents.len()) {
            return bad(format!("1 to 5 opponents are supported, got {}", self.opponents.len()));
        }
        if !self.leaks.is_empty() && self.leaks.len() != self.opponents.len() {
            return bad(format!("give one leak per opponent ({}), or none", self.opponents.len()));
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
            Format::SitAndGo { starting_chips, levels, hands_per_level, payouts } => {
                if !payouts.is_empty() {
                    if payouts.len() > self.seats() {
                        return bad(format!("{} paid places for {} players", payouts.len(), self.seats()));
                    }
                    if payouts.contains(&0) || payouts.windows(2).any(|w| w[1] > w[0]) {
                        return bad("payouts must be positive and must not increase from 1st place down".into());
                    }
                    if payouts.iter().sum::<u32>() != 100 {
                        return bad("payouts are percentages of the prize pool and must add up to 100".into());
                    }
                }
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
            leaks: vec![],
        };
        cash(6, 5).validate().unwrap();
        cash(2, 1).validate().unwrap();
        assert!(cash(6, 4).validate().is_err(), "seat count mismatch");
        assert!(cash(6, 0).validate().is_err());
        let sng = |levels: Vec<Level>, chips: u64| TableConfig {
            format: Format::SitAndGo { starting_chips: chips, levels, hands_per_level: 10, payouts: vec![] },
            opponents: vec![Difficulty::Easy; 2],
            seed: 1,
            leaks: vec![],
        };
        let l = |s, b| Level { small_blind: s, big_blind: b, ante: 0 };
        sng(vec![l(10, 20), l(15, 30)], 1500).validate().unwrap();
        assert!(sng(vec![l(10, 20), l(5, 10)], 1500).validate().is_err());
        assert!(sng(vec![l(10, 20)], 100).validate().is_err());
        assert!(sng(vec![l(30, 20)], 1500).validate().is_err());
        let paid = |payouts: Vec<u32>| TableConfig {
            format: Format::SitAndGo { starting_chips: 1500, levels: vec![l(10, 20)], hands_per_level: 10, payouts },
            opponents: vec![Difficulty::Easy; 3],
            seed: 1,
            leaks: vec![],
        };
        paid(vec![65, 35]).validate().unwrap();
        paid(vec![50, 30, 20]).validate().unwrap();
        assert!(paid(vec![60, 30]).validate().is_err(), "must add up to 100");
        assert!(paid(vec![30, 70]).validate().is_err(), "must not increase");
        assert!(paid(vec![40, 30, 20, 5, 5]).validate().is_err(), "more places than players");
        assert_eq!(default_payouts(3), vec![100]);
        assert_eq!(default_payouts(6), vec![65, 35]);
        let json = serde_json::to_value(cash(2, 1)).unwrap();
        assert_eq!(json["format"]["type"], "cash");
        assert_eq!(serde_json::from_value::<TableConfig>(json).unwrap(), cash(2, 1));
    }
}
