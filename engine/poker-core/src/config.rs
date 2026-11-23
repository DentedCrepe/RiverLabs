//! Game configuration: table size, blinds, stack depth, rake and (optional) currency stake.

use crate::chips::{Chips, MBB_PER_BB};
use crate::error::{PokerError, Result};
use crate::position::{MAX_PLAYERS, MIN_PLAYERS};
use crate::rake::RakeConfig;
use crate::stack::StackConfig;
use serde::{Deserialize, Serialize};

pub const GAME_CONFIG_SCHEMA_VERSION: u32 = 1;

/// Currency stake, needed only when rake amounts are expressed in currency.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stake {
    /// ISO 4217 code, e.g. "USD".
    pub currency: String,
    /// Big blind in currency minor units (e.g. 100 = $1.00).
    pub big_blind_minor_units: u64,
}

impl Stake {
    pub fn validate(&self) -> Result<()> {
        if self.currency.len() != 3 || !self.currency.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(PokerError::InvalidConfig(format!(
                "currency must be a 3-letter ISO 4217 code, got '{}'",
                self.currency
            )));
        }
        if self.big_blind_minor_units == 0 {
            return Err(PokerError::InvalidConfig("big blind must be positive".into()));
        }
        Ok(())
    }
}

/// No-limit hold'em cash-game configuration. The big blind is the unit (1000 mbb) by
/// definition; the small blind and ante are expressed relative to it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameConfig {
    pub schema_version: u32,
    pub players: u8,
    pub small_blind: Chips,
    /// Per-player ante (dead money). 0 for standard cash games.
    pub ante: Chips,
    pub stack: StackConfig,
    pub rake: RakeConfig,
    pub stake: Option<Stake>,
}

impl Default for GameConfig {
    /// Six-max, 0.5/1 blinds, no ante, 100bb, default rake (5%, NFND, uncapped).
    fn default() -> Self {
        GameConfig {
            schema_version: GAME_CONFIG_SCHEMA_VERSION,
            players: 6,
            small_blind: Chips(MBB_PER_BB / 2),
            ante: Chips::ZERO,
            stack: StackConfig::default(),
            rake: RakeConfig::default(),
            stake: None,
        }
    }
}

impl GameConfig {
    pub const BIG_BLIND: Chips = Chips(MBB_PER_BB);

    pub fn validate(&self) -> Result<()> {
        if self.schema_version != GAME_CONFIG_SCHEMA_VERSION {
            return Err(PokerError::InvalidConfig(format!(
                "unsupported game config schema version {} (expected {GAME_CONFIG_SCHEMA_VERSION})",
                self.schema_version
            )));
        }
        let n = self.players as usize;
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&n) {
            return Err(PokerError::InvalidConfig(format!(
                "players must be between {MIN_PLAYERS} and {MAX_PLAYERS}, got {n}"
            )));
        }
        if self.small_blind.is_zero() || self.small_blind > Self::BIG_BLIND {
            return Err(PokerError::InvalidConfig(format!(
                "small blind must be in (0, 1bb], got {}",
                self.small_blind
            )));
        }
        if self.ante > Self::BIG_BLIND {
            return Err(PokerError::InvalidConfig(format!("ante must be at most 1bb, got {}", self.ante)));
        }
        self.stack.validate()?;
        if let Some(s) = &self.stake {
            s.validate()?;
        }
        self.rake.validate(self.stake.as_ref())?;
        // Every player must be able to post ante + big blind and keep chips behind.
        if self.stack.effective_stack <= self.ante + Self::BIG_BLIND {
            return Err(PokerError::InvalidConfig("effective stack must exceed ante plus big blind".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rake::RakeAmount;

    #[test]
    fn default_config_is_valid_six_max_100bb() {
        let c = GameConfig::default();
        c.validate().unwrap();
        assert_eq!(c.players, 6);
        assert_eq!(c.stack.effective_stack, Chips(100_000));
        assert_eq!(c.rake.percentage_bps, 500);
    }

    #[test]
    fn rejects_bad_values() {
        let base = GameConfig::default();
        for bad in [
            GameConfig { players: 1, ..base.clone() },
            GameConfig { players: 7, ..base.clone() },
            GameConfig { small_blind: Chips::ZERO, ..base.clone() },
            GameConfig { small_blind: Chips(1_001), ..base.clone() },
            GameConfig { ante: Chips(1_001), ..base.clone() },
            GameConfig { schema_version: 99, ..base.clone() },
            GameConfig { stake: Some(Stake { currency: "usd".into(), big_blind_minor_units: 100 }), ..base.clone() },
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn rake_and_stack_are_independent() {
        let mut c = GameConfig { stack: StackConfig::custom(Chips(37_500)).unwrap(), ..GameConfig::default() };
        c.rake.cap = Some(RakeAmount::Currency { minor_units: 300 });
        assert!(c.validate().is_err(), "currency cap needs stake");
        c.stake = Some(Stake { currency: "USD".into(), big_blind_minor_units: 100 });
        c.validate().unwrap();
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<GameConfig>(&json).unwrap(), c);
    }
}
