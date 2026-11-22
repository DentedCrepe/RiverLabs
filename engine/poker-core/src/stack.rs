//! Effective stack configuration. Independent of rake configuration (see ADR-0004).

use crate::chips::{Chips, MBB_PER_BB};
use crate::error::{PokerError, Result};
use serde::{Deserialize, Serialize};

/// Smallest supported effective stack (2bb): every player must be able to post a blind and
/// still hold chips. Largest: 1000bb.
pub const MIN_EFFECTIVE_STACK: Chips = Chips(2 * MBB_PER_BB);
pub const MAX_EFFECTIVE_STACK: Chips = Chips(1000 * MBB_PER_BB);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StackPreset {
    #[serde(rename = "20bb")]
    Bb20,
    #[serde(rename = "40bb")]
    Bb40,
    #[serde(rename = "50bb")]
    Bb50,
    #[serde(rename = "75bb")]
    Bb75,
    #[serde(rename = "100bb")]
    Bb100,
    #[serde(rename = "150bb")]
    Bb150,
    #[serde(rename = "200bb")]
    Bb200,
}

impl StackPreset {
    pub const ALL: [StackPreset; 7] = [
        StackPreset::Bb20,
        StackPreset::Bb40,
        StackPreset::Bb50,
        StackPreset::Bb75,
        StackPreset::Bb100,
        StackPreset::Bb150,
        StackPreset::Bb200,
    ];

    pub const fn big_blinds(self) -> u64 {
        match self {
            StackPreset::Bb20 => 20,
            StackPreset::Bb40 => 40,
            StackPreset::Bb50 => 50,
            StackPreset::Bb75 => 75,
            StackPreset::Bb100 => 100,
            StackPreset::Bb150 => 150,
            StackPreset::Bb200 => 200,
        }
    }

    pub const fn chips(self) -> Chips {
        Chips(self.big_blinds() * MBB_PER_BB)
    }

    pub const fn label(self) -> &'static str {
        match self {
            StackPreset::Bb20 => "20bb",
            StackPreset::Bb40 => "40bb",
            StackPreset::Bb50 => "50bb",
            StackPreset::Bb75 => "75bb",
            StackPreset::Bb100 => "100bb",
            StackPreset::Bb150 => "150bb",
            StackPreset::Bb200 => "200bb",
        }
    }
}

/// The effective stack every player starts with. Custom depths are any value in
/// `[MIN_EFFECTIVE_STACK, MAX_EFFECTIVE_STACK]` at 1 mbb resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StackConfig {
    pub effective_stack: Chips,
}

impl Default for StackConfig {
    /// 100bb.
    fn default() -> Self {
        StackConfig::from_preset(StackPreset::Bb100)
    }
}

impl StackConfig {
    pub const fn from_preset(p: StackPreset) -> StackConfig {
        StackConfig { effective_stack: p.chips() }
    }

    pub fn custom(effective_stack: Chips) -> Result<StackConfig> {
        let c = StackConfig { effective_stack };
        c.validate()?;
        Ok(c)
    }

    pub fn validate(&self) -> Result<()> {
        if self.effective_stack < MIN_EFFECTIVE_STACK || self.effective_stack > MAX_EFFECTIVE_STACK {
            return Err(PokerError::InvalidConfig(format!(
                "effective stack must be between {MIN_EFFECTIVE_STACK} and {MAX_EFFECTIVE_STACK}, got {}",
                self.effective_stack
            )));
        }
        Ok(())
    }

    /// The preset this configuration corresponds to, if any.
    pub fn preset(&self) -> Option<StackPreset> {
        StackPreset::ALL.into_iter().find(|p| p.chips() == self.effective_stack)
    }
}

/// Effective stack for `hero` against the other listed live stacks: the most `hero` can win
/// from or lose to any single opponent, i.e. `min(hero, max(opponents))`.
/// Returns `None` if there are no opponents.
pub fn effective_stack(hero: Chips, opponents: &[Chips]) -> Option<Chips> {
    opponents.iter().copied().max().map(|m| hero.min(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_100bb() {
        assert_eq!(StackConfig::default().effective_stack, Chips(100_000));
        assert_eq!(StackConfig::default().preset(), Some(StackPreset::Bb100));
    }

    #[test]
    fn presets_are_exact() {
        let bbs: Vec<u64> = StackPreset::ALL.iter().map(|p| p.big_blinds()).collect();
        assert_eq!(bbs, vec![20, 40, 50, 75, 100, 150, 200]);
        for p in StackPreset::ALL {
            assert_eq!(StackConfig::from_preset(p).preset(), Some(p));
            StackConfig::from_preset(p).validate().unwrap();
        }
    }

    #[test]
    fn custom_depth_bounds() {
        assert!(StackConfig::custom(Chips(62_500)).unwrap().preset().is_none());
        assert!(StackConfig::custom(Chips(1_999)).is_err());
        assert!(StackConfig::custom(Chips(1_000_001)).is_err());
    }

    #[test]
    fn effective_stack_is_min_of_hero_and_biggest_opponent() {
        assert_eq!(effective_stack(Chips(100), &[Chips(50), Chips(80)]), Some(Chips(80)));
        assert_eq!(effective_stack(Chips(60), &[Chips(50), Chips(80)]), Some(Chips(60)));
        assert_eq!(effective_stack(Chips(60), &[]), None);
    }

    #[test]
    fn serde_preset_labels() {
        assert_eq!(serde_json::to_string(&StackPreset::Bb75).unwrap(), "\"75bb\"");
        let c: StackConfig = serde_json::from_str(r#"{"effective_stack":40000}"#).unwrap();
        assert_eq!(c.preset(), Some(StackPreset::Bb40));
    }
}
