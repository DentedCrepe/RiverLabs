//! Integer chip amounts in milli-big-blinds (ADR-0002).

use crate::error::{PokerError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// Milli-big-blinds per big blind.
pub const MBB_PER_BB: u64 = 1000;

/// A non-negative chip amount in milli-big-blinds. Arithmetic panics on underflow in debug
/// and saturating behaviour is never used: callers use [`Chips::checked_sub`] where an
/// underflow is a possible input error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Chips(pub u64);

impl Chips {
    pub const ZERO: Chips = Chips(0);
    pub const ONE_BB: Chips = Chips(MBB_PER_BB);

    pub const fn from_mbb(mbb: u64) -> Chips {
        Chips(mbb)
    }
    pub fn from_bb(bb: u64) -> Result<Chips> {
        bb.checked_mul(MBB_PER_BB).map(Chips).ok_or(PokerError::Overflow)
    }
    pub const fn mbb(self) -> u64 {
        self.0
    }
    /// Lossy conversion for display only. Never feed the result back into chip arithmetic.
    pub fn as_bb_f64(self) -> f64 {
        self.0 as f64 / MBB_PER_BB as f64
    }
    pub fn checked_sub(self, o: Chips) -> Option<Chips> {
        self.0.checked_sub(o.0).map(Chips)
    }
    pub fn checked_add(self, o: Chips) -> Option<Chips> {
        self.0.checked_add(o.0).map(Chips)
    }
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl Add for Chips {
    type Output = Chips;
    fn add(self, o: Chips) -> Chips {
        Chips(self.0.checked_add(o.0).expect("chip overflow"))
    }
}
impl AddAssign for Chips {
    fn add_assign(&mut self, o: Chips) {
        *self = *self + o;
    }
}
impl Sub for Chips {
    type Output = Chips;
    fn sub(self, o: Chips) -> Chips {
        Chips(self.0.checked_sub(o.0).expect("chip underflow"))
    }
}
impl SubAssign for Chips {
    fn sub_assign(&mut self, o: Chips) {
        *self = *self - o;
    }
}
impl Sum for Chips {
    fn sum<I: Iterator<Item = Chips>>(iter: I) -> Chips {
        iter.fold(Chips::ZERO, |a, b| a + b)
    }
}

impl fmt::Display for Chips {
    /// Exact decimal rendering in big blinds, e.g. `2.5bb`, `0.001bb`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.0 / MBB_PER_BB;
        let frac = self.0 % MBB_PER_BB;
        if frac == 0 {
            write!(f, "{whole}bb")
        } else {
            let s = format!("{frac:03}");
            write!(f, "{whole}.{}bb", s.trim_end_matches('0'))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_exact() {
        assert_eq!(Chips(2500).to_string(), "2.5bb");
        assert_eq!(Chips(100_000).to_string(), "100bb");
        assert_eq!(Chips(1).to_string(), "0.001bb");
        assert_eq!(Chips(1050).to_string(), "1.05bb");
    }

    #[test]
    fn from_bb_overflow_is_reported() {
        assert_eq!(Chips::from_bb(100).unwrap(), Chips(100_000));
        assert_eq!(Chips::from_bb(u64::MAX), Err(PokerError::Overflow));
    }
}
