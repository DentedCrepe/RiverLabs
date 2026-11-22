//! Rake configuration and calculation. Rules and assumptions are documented in `docs/rake.md`.
//!
//! All arithmetic is exact integer/rational arithmetic on milli-big-blinds. Currency amounts
//! are converted through the configured [`Stake`]; conversions that are not an exact number of
//! mbb are rejected during validation instead of being rounded silently.

use crate::chips::{Chips, MBB_PER_BB};
use crate::config::Stake;
use crate::error::{PokerError, Result};
use serde::{Deserialize, Serialize};

/// Basis points per 100%.
pub const BPS_PER_UNIT: u32 = 10_000;

/// An amount expressed either in big blinds (as mbb) or in currency minor units (e.g. cents).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "unit", rename_all = "snake_case", deny_unknown_fields)]
pub enum RakeAmount {
    BigBlinds { mbb: u64 },
    Currency { minor_units: u64 },
}

impl RakeAmount {
    /// Resolve to mbb, requiring an exact conversion.
    pub fn to_chips(self, stake: Option<&Stake>, what: &str) -> Result<Chips> {
        match self {
            RakeAmount::BigBlinds { mbb } => Ok(Chips(mbb)),
            RakeAmount::Currency { minor_units } => {
                let stake = stake.ok_or_else(|| {
                    PokerError::InvalidConfig(format!(
                        "{what} is expressed in currency but the game has no stake configured"
                    ))
                })?;
                let num = (minor_units as u128) * (MBB_PER_BB as u128);
                let den = stake.big_blind_minor_units as u128;
                if num % den != 0 {
                    return Err(PokerError::InvalidConfig(format!(
                        "{what} of {minor_units} minor units is not an exact number of milli-big-blinds \
                         at a big blind of {} minor units",
                        stake.big_blind_minor_units
                    )));
                }
                u64::try_from(num / den).map(Chips).map_err(|_| PokerError::Overflow)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoundingMode {
    /// Round down to a multiple of the increment (most common: rake is truncated).
    #[default]
    Down,
    /// Round half up to the nearest multiple of the increment.
    Nearest,
    /// Round up to a multiple of the increment (never above the cap or the pot).
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RakeRounding {
    pub increment: RakeAmount,
    pub mode: RoundingMode,
}

impl Default for RakeRounding {
    /// Finest resolution (1 mbb), rounding down.
    fn default() -> Self {
        RakeRounding { increment: RakeAmount::BigBlinds { mbb: 1 }, mode: RoundingMode::Down }
    }
}

/// How the total rake is taken from the main pot and side pots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RakeAllocation {
    /// Each pot pays rake in proportion to its size (largest-remainder rounding, ties to
    /// the main pot first).
    #[default]
    Proportional,
    /// Rake is taken from the main pot first, then from side pots in order.
    MainPotFirst,
}

/// Rake structure. Independent of the effective stack configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RakeConfig {
    /// Rake percentage in basis points (500 = 5%). Range 0..=10000.
    pub percentage_bps: u32,
    /// Maximum rake per hand. `None` = uncapped.
    pub cap: Option<RakeAmount>,
    /// No rake is taken when the final pot is below this amount. `None` = no threshold.
    pub min_pot: Option<RakeAmount>,
    pub rounding: RakeRounding,
    /// When true, hands that end before the flop is dealt are not raked.
    pub no_flop_no_drop: bool,
    pub allocation: RakeAllocation,
}

impl Default for RakeConfig {
    /// 5%, uncapped, no minimum pot, 1 mbb rounding down, no-flop-no-drop, proportional.
    fn default() -> Self {
        RakeConfig {
            percentage_bps: 500,
            cap: None,
            min_pot: None,
            rounding: RakeRounding::default(),
            no_flop_no_drop: true,
            allocation: RakeAllocation::Proportional,
        }
    }
}

impl RakeConfig {
    /// A rake-free configuration.
    pub fn none() -> RakeConfig {
        RakeConfig { percentage_bps: 0, ..RakeConfig::default() }
    }

    pub fn validate(&self, stake: Option<&Stake>) -> Result<()> {
        self.resolve(stake).map(|_| ())
    }

    fn resolve(&self, stake: Option<&Stake>) -> Result<ResolvedRake> {
        if self.percentage_bps > BPS_PER_UNIT {
            return Err(PokerError::InvalidConfig(format!(
                "rake percentage must be between 0 and 10000 basis points, got {}",
                self.percentage_bps
            )));
        }
        let increment = self.rounding.increment.to_chips(stake, "rake rounding increment")?;
        if increment.is_zero() {
            return Err(PokerError::InvalidConfig("rake rounding increment must be positive".into()));
        }
        let cap = self.cap.map(|c| c.to_chips(stake, "rake cap")).transpose()?;
        if let Some(cap) = cap {
            if cap.mbb() % increment.mbb() != 0 {
                return Err(PokerError::InvalidConfig(format!(
                    "rake cap {cap} must be a multiple of the rounding increment {increment}"
                )));
            }
        }
        let min_pot = self.min_pot.map(|m| m.to_chips(stake, "minimum raked pot")).transpose()?;
        Ok(ResolvedRake { increment, cap, min_pot })
    }

    /// Rake for a hand whose final pot (after uncalled bets are returned) is `pot`.
    pub fn compute(&self, pot: Chips, saw_flop: bool, stake: Option<&Stake>) -> Result<RakeResult> {
        let r = self.resolve(stake)?;
        let zero = |reason| Ok(RakeResult { rake: Chips::ZERO, capped: false, reason });
        if self.no_flop_no_drop && !saw_flop {
            return zero(RakeReason::NoFlopNoDrop);
        }
        if let Some(min) = r.min_pot {
            if pot < min {
                return zero(RakeReason::BelowMinimumPot);
            }
        }
        if self.percentage_bps == 0 || pot.is_zero() {
            return zero(RakeReason::ZeroRake);
        }
        // raw rake = pot * bps / 10000, kept as the rational n / d.
        let n = pot.mbb() as u128 * self.percentage_bps as u128;
        let d = BPS_PER_UNIT as u128;
        if let Some(cap) = r.cap {
            if n >= cap.mbb() as u128 * d {
                return Ok(RakeResult { rake: cap.min(pot), capped: true, reason: RakeReason::Capped });
            }
        }
        let inc = r.increment.mbb() as u128;
        let di = d * inc;
        let multiples = match self.rounding.mode {
            RoundingMode::Down => n / di,
            RoundingMode::Up => n.div_ceil(di),
            RoundingMode::Nearest => (2 * n + di) / (2 * di),
        };
        let mut rake = u64::try_from(multiples * inc).map_err(|_| PokerError::Overflow)?;
        if let Some(cap) = r.cap {
            rake = rake.min(cap.mbb());
        }
        rake = rake.min(pot.mbb());
        Ok(RakeResult { rake: Chips(rake), capped: false, reason: RakeReason::Percentage })
    }

    /// Splits `rake` across pots according to [`RakeAllocation`]. `pots[0]` is the main pot.
    /// The returned shares sum to exactly `rake` and never exceed a pot's size.
    pub fn allocate(&self, rake: Chips, pots: &[Chips]) -> Result<Vec<Chips>> {
        let total: Chips = pots.iter().copied().sum();
        if rake > total {
            return Err(PokerError::InvalidTransition(format!("rake {rake} exceeds total pot {total}")));
        }
        if rake.is_zero() {
            return Ok(vec![Chips::ZERO; pots.len()]);
        }
        match self.allocation {
            RakeAllocation::MainPotFirst => {
                let mut left = rake;
                Ok(pots
                    .iter()
                    .map(|&p| {
                        let take = p.min(left);
                        left -= take;
                        take
                    })
                    .collect())
            }
            RakeAllocation::Proportional => {
                let (r, t) = (rake.mbb() as u128, total.mbb() as u128);
                let mut shares: Vec<u64> = Vec::with_capacity(pots.len());
                let mut rems: Vec<(u128, usize)> = Vec::with_capacity(pots.len());
                for (i, p) in pots.iter().enumerate() {
                    let num = r * p.mbb() as u128;
                    shares.push((num / t) as u64);
                    rems.push((num % t, i));
                }
                let assigned: u64 = shares.iter().sum();
                let mut missing = rake.mbb() - assigned;
                // Largest remainder first; ties go to the earlier (main) pot.
                rems.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                for &(_, i) in &rems {
                    if missing == 0 {
                        break;
                    }
                    if shares[i] < pots[i].mbb() {
                        shares[i] += 1;
                        missing -= 1;
                    }
                }
                debug_assert_eq!(missing, 0);
                Ok(shares.into_iter().map(Chips).collect())
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedRake {
    increment: Chips,
    cap: Option<Chips>,
    min_pot: Option<Chips>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RakeReason {
    NoFlopNoDrop,
    BelowMinimumPot,
    ZeroRake,
    Percentage,
    Capped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RakeResult {
    pub rake: Chips,
    pub capped: bool,
    pub reason: RakeReason,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn bb(x: u64) -> RakeAmount {
        RakeAmount::BigBlinds { mbb: x * 1000 }
    }
    fn stake_cents(bb: u64) -> Stake {
        Stake { currency: "USD".into(), big_blind_minor_units: bb }
    }

    #[test]
    fn default_is_five_percent_with_nfnd() {
        let c = RakeConfig::default();
        assert_eq!(c.percentage_bps, 500);
        assert!(c.no_flop_no_drop);
        let r = c.compute(Chips(10_000), true, None).unwrap();
        assert_eq!(r.rake, Chips(500));
        assert_eq!(r.reason, RakeReason::Percentage);
        let r = c.compute(Chips(10_000), false, None).unwrap();
        assert_eq!((r.rake, r.reason), (Chips::ZERO, RakeReason::NoFlopNoDrop));
    }

    #[test]
    fn cap_in_big_blinds() {
        let c = RakeConfig { cap: Some(bb(3)), ..RakeConfig::default() };
        // 5% of 100bb = 5bb > 3bb cap
        let r = c.compute(Chips(100_000), true, None).unwrap();
        assert_eq!((r.rake, r.capped), (Chips(3_000), true));
        // exactly at the cap: 5% of 60bb = 3bb
        let r = c.compute(Chips(60_000), true, None).unwrap();
        assert_eq!((r.rake, r.capped), (Chips(3_000), true));
        let r = c.compute(Chips(59_999), true, None).unwrap();
        assert!(!r.capped);
        assert_eq!(r.rake, Chips(2_999)); // 2999.95 rounded down
    }

    #[test]
    fn cap_in_currency_requires_stake_and_exact_conversion() {
        let c = RakeConfig { cap: Some(RakeAmount::Currency { minor_units: 300 }), ..RakeConfig::default() };
        assert!(c.validate(None).is_err());
        // $0.50/$1.00: $3.00 cap = 3bb
        let s = stake_cents(100);
        let r = c.compute(Chips(100_000), true, Some(&s)).unwrap();
        assert_eq!(r.rake, Chips(3_000));
        // $0.25 big blind: $3.00 = 12bb
        let r = c.compute(Chips(1_000_000), true, Some(&stake_cents(25))).unwrap();
        assert_eq!(r.rake, Chips(12_000));
        // 3-cent big blind: 1 cent = 333.33 mbb, not exact → rejected
        let c2 = RakeConfig { cap: Some(RakeAmount::Currency { minor_units: 1 }), ..RakeConfig::default() };
        assert!(c2.validate(Some(&stake_cents(3))).is_err());
    }

    #[test]
    fn minimum_pot_threshold() {
        let c = RakeConfig { min_pot: Some(bb(2)), ..RakeConfig::default() };
        assert_eq!(c.compute(Chips(1_999), true, None).unwrap().reason, RakeReason::BelowMinimumPot);
        assert_eq!(c.compute(Chips(2_000), true, None).unwrap().rake, Chips(100));
    }

    #[test]
    fn rounding_modes_on_a_cent_increment() {
        // $1 big blind => 1 cent = 10 mbb. Pot 1.23bb => 5% = 61.5 mbb.
        let s = stake_cents(100);
        let mk = |mode| RakeConfig {
            rounding: RakeRounding { increment: RakeAmount::Currency { minor_units: 1 }, mode },
            ..RakeConfig::default()
        };
        let pot = Chips(1_230);
        assert_eq!(mk(RoundingMode::Down).compute(pot, true, Some(&s)).unwrap().rake, Chips(60));
        assert_eq!(mk(RoundingMode::Nearest).compute(pot, true, Some(&s)).unwrap().rake, Chips(60));
        assert_eq!(mk(RoundingMode::Up).compute(pot, true, Some(&s)).unwrap().rake, Chips(70));
        // exact half: pot 1.3bb => 65 mbb => nearest rounds half up to 70
        assert_eq!(mk(RoundingMode::Nearest).compute(Chips(1_300), true, Some(&s)).unwrap().rake, Chips(70));
    }

    #[test]
    fn nfnd_disabled_rakes_preflop_hands() {
        let c = RakeConfig { no_flop_no_drop: false, ..RakeConfig::default() };
        assert_eq!(c.compute(Chips(2_000), false, None).unwrap().rake, Chips(100));
    }

    #[test]
    fn invalid_configs_are_rejected() {
        let c = RakeConfig { percentage_bps: 10_001, ..RakeConfig::default() };
        assert!(c.validate(None).is_err());
        let c = RakeConfig {
            rounding: RakeRounding { increment: RakeAmount::BigBlinds { mbb: 0 }, mode: RoundingMode::Down },
            ..RakeConfig::default()
        };
        assert!(c.validate(None).is_err());
        let c = RakeConfig {
            cap: Some(RakeAmount::BigBlinds { mbb: 2_505 }),
            rounding: RakeRounding { increment: RakeAmount::BigBlinds { mbb: 10 }, mode: RoundingMode::Down },
            ..RakeConfig::default()
        };
        assert!(c.validate(None).is_err(), "cap not a multiple of increment");
    }

    #[test]
    fn rake_never_exceeds_pot_with_round_up() {
        let c = RakeConfig {
            percentage_bps: 10_000,
            rounding: RakeRounding { increment: RakeAmount::BigBlinds { mbb: 100 }, mode: RoundingMode::Up },
            ..RakeConfig::default()
        };
        assert_eq!(c.compute(Chips(1), true, None).unwrap().rake, Chips(1));
    }

    #[test]
    fn allocation_proportional_and_main_first() {
        let c = RakeConfig::default();
        let shares = c.allocate(Chips(100), &[Chips(300), Chips(100)]).unwrap();
        assert_eq!(shares, vec![Chips(75), Chips(25)]);
        // 10 across three equal pots: 4,3,3 (tie goes to main pot)
        let shares = c.allocate(Chips(10), &[Chips(30), Chips(30), Chips(30)]).unwrap();
        assert_eq!(shares, vec![Chips(4), Chips(3), Chips(3)]);
        let m = RakeConfig { allocation: RakeAllocation::MainPotFirst, ..c };
        assert_eq!(m.allocate(Chips(50), &[Chips(30), Chips(40)]).unwrap(), vec![Chips(30), Chips(20)]);
        assert!(c.allocate(Chips(500), &[Chips(100)]).is_err());
    }

    #[test]
    fn serde_round_trip_and_unknown_fields_rejected() {
        let c = RakeConfig { cap: Some(RakeAmount::Currency { minor_units: 300 }), ..RakeConfig::default() };
        let s = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<RakeConfig>(&s).unwrap(), c);
        let bad = s.replace("\"no_flop_no_drop\"", "\"bogus\":1,\"no_flop_no_drop\"");
        assert!(serde_json::from_str::<RakeConfig>(&bad).is_err());
    }

    proptest! {
        #[test]
        fn rake_bounded_and_monotone(pot in 0u64..10_000_000, bps in 0u32..=10_000,
                                     cap in proptest::option::of(1u64..100_000), inc in 1u64..50,
                                     mode in prop_oneof![Just(RoundingMode::Down), Just(RoundingMode::Nearest), Just(RoundingMode::Up)]) {
            let cap = cap.map(|c| c * inc);
            let c = RakeConfig {
                percentage_bps: bps,
                cap: cap.map(|m| RakeAmount::BigBlinds { mbb: m }),
                rounding: RakeRounding { increment: RakeAmount::BigBlinds { mbb: inc }, mode },
                ..RakeConfig::default()
            };
            let r = c.compute(Chips(pot), true, None).unwrap().rake.mbb();
            prop_assert!(r <= pot);
            if let Some(cap) = cap { prop_assert!(r <= cap); }
            // Within one increment of the exact percentage (unless capped / clamped)
            let exact = pot as f64 * bps as f64 / 10_000.0;
            if cap.is_none_or(|c| (c as f64) > exact) {
                prop_assert!((r as f64 - exact).abs() <= inc as f64 + 1e-9 || r == pot);
            }
            let r2 = c.compute(Chips(pot + 1), true, None).unwrap().rake.mbb();
            prop_assert!(r2 >= r, "rake must be monotone in pot size");
        }

        #[test]
        fn allocation_conserves(rake_frac in 0.0f64..1.0, pots in proptest::collection::vec(1u64..1_000_000, 1..6),
                                main_first in any::<bool>()) {
            let total: u64 = pots.iter().sum();
            let rake = Chips((total as f64 * rake_frac) as u64);
            let mut c = RakeConfig::default();
            if main_first { c.allocation = RakeAllocation::MainPotFirst; }
            let pots: Vec<Chips> = pots.into_iter().map(Chips).collect();
            let shares = c.allocate(rake, &pots).unwrap();
            prop_assert_eq!(shares.iter().copied().sum::<Chips>(), rake);
            for (s, p) in shares.iter().zip(&pots) { prop_assert!(s <= p); }
        }
    }
}
