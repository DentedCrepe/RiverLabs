use crate::combo::Combo;
use poker_core::{Card, PokerError, Rank, Suit};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandKind {
    Pair,
    Suited,
    Offsuit,
}

/// One of the 169 strategically distinct starting-hand classes (e.g. `AKs`, `T9o`, `77`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandClass {
    high: Rank,
    low: Rank,
    kind: HandKind,
}

impl HandClass {
    pub fn new(a: Rank, b: Rank, kind: HandKind) -> Result<HandClass, PokerError> {
        let (high, low) = if a >= b { (a, b) } else { (b, a) };
        match (high == low, kind) {
            (true, HandKind::Pair) | (false, HandKind::Suited | HandKind::Offsuit) => Ok(HandClass { high, low, kind }),
            _ => Err(PokerError::InvalidConfig(format!("{}{} cannot be {kind:?}", high.to_char(), low.to_char()))),
        }
    }

    pub fn high(self) -> Rank {
        self.high
    }
    pub fn low(self) -> Rank {
        self.low
    }
    pub fn kind(self) -> HandKind {
        self.kind
    }

    /// Class of a specific combo.
    pub fn of(combo: Combo) -> HandClass {
        let (a, b) = combo.cards();
        let kind = if a.rank() == b.rank() {
            HandKind::Pair
        } else if a.suit() == b.suit() {
            HandKind::Suited
        } else {
            HandKind::Offsuit
        };
        HandClass::new(a.rank(), b.rank(), kind).expect("consistent")
    }

    /// All 169 classes in matrix order (row-major, aces top-left).
    pub fn all() -> Vec<HandClass> {
        let mut v = Vec::with_capacity(169);
        for row in 0..13 {
            for col in 0..13 {
                v.push(HandClass::at(row, col));
            }
        }
        v
    }

    /// Standard 13×13 grid: row/col 0 = ace … 12 = deuce; pairs on the diagonal, suited
    /// hands above it (row < col), offsuit hands below.
    pub fn at(row: usize, col: usize) -> HandClass {
        assert!(row < 13 && col < 13, "grid is 13x13");
        let r = |i: usize| Rank::from_index(12 - i as u8).expect("<13");
        match row.cmp(&col) {
            std::cmp::Ordering::Equal => HandClass { high: r(row), low: r(row), kind: HandKind::Pair },
            std::cmp::Ordering::Less => HandClass { high: r(row), low: r(col), kind: HandKind::Suited },
            std::cmp::Ordering::Greater => HandClass { high: r(col), low: r(row), kind: HandKind::Offsuit },
        }
    }

    /// `(row, col)` in the 13×13 grid.
    pub fn grid_position(self) -> (usize, usize) {
        let (h, l) = (12 - self.high.index() as usize, 12 - self.low.index() as usize);
        match self.kind {
            HandKind::Pair | HandKind::Suited => (h, l),
            HandKind::Offsuit => (l, h),
        }
    }

    /// The specific combos in this class: 6 for pairs, 4 suited, 12 offsuit.
    pub fn combos(self) -> Vec<Combo> {
        let mut v = Vec::with_capacity(12);
        for s1 in Suit::ALL {
            for s2 in Suit::ALL {
                let (a, b) = (Card::new(self.high, s1), Card::new(self.low, s2));
                let ok = match self.kind {
                    HandKind::Pair => s1 < s2,
                    HandKind::Suited => s1 == s2,
                    HandKind::Offsuit => s1 != s2,
                };
                if ok {
                    v.push(Combo::new(a, b).expect("distinct"));
                }
            }
        }
        v
    }

    pub fn combo_count(self) -> usize {
        match self.kind {
            HandKind::Pair => 6,
            HandKind::Suited => 4,
            HandKind::Offsuit => 12,
        }
    }
}

impl fmt::Display for HandClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = match self.kind {
            HandKind::Pair => "",
            HandKind::Suited => "s",
            HandKind::Offsuit => "o",
        };
        write!(f, "{}{}{}", self.high.to_char(), self.low.to_char(), suffix)
    }
}

impl FromStr for HandClass {
    type Err = PokerError;
    fn from_str(s: &str) -> Result<HandClass, PokerError> {
        let c: Vec<char> = s.chars().collect();
        let bad = || PokerError::InvalidConfig(format!("invalid hand class '{s}'"));
        let (a, b) = match c.as_slice() {
            [a, b] | [a, b, _] => (Rank::from_char(*a).map_err(|_| bad())?, Rank::from_char(*b).map_err(|_| bad())?),
            _ => return Err(bad()),
        };
        let kind = match (c.get(2).map(|x| x.to_ascii_lowercase()), a == b) {
            (None, true) => HandKind::Pair,
            (Some('s'), false) => HandKind::Suited,
            (Some('o'), false) => HandKind::Offsuit,
            _ => return Err(bad()),
        };
        HandClass::new(a, b, kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combo::COMBO_COUNT;

    #[test]
    fn there_are_169_classes_covering_1326_combos_exactly_once() {
        let all = HandClass::all();
        assert_eq!(all.len(), 169);
        let mut seen = vec![0u8; COMBO_COUNT];
        for c in &all {
            assert_eq!(c.combos().len(), c.combo_count());
            for combo in c.combos() {
                seen[combo.index()] += 1;
                assert_eq!(HandClass::of(combo), *c);
            }
        }
        assert!(seen.iter().all(|&n| n == 1));
        let pairs = all.iter().filter(|c| c.kind() == HandKind::Pair).count();
        let suited = all.iter().filter(|c| c.kind() == HandKind::Suited).count();
        assert_eq!((pairs, suited, 169 - pairs - suited), (13, 78, 78));
    }

    #[test]
    fn grid_layout() {
        assert_eq!(HandClass::at(0, 0).to_string(), "AA");
        assert_eq!(HandClass::at(0, 1).to_string(), "AKs");
        assert_eq!(HandClass::at(1, 0).to_string(), "AKo");
        assert_eq!(HandClass::at(12, 12).to_string(), "22");
        assert_eq!(HandClass::at(11, 12).to_string(), "32s");
        for row in 0..13 {
            for col in 0..13 {
                assert_eq!(HandClass::at(row, col).grid_position(), (row, col));
            }
        }
    }

    #[test]
    fn parse() {
        assert_eq!("AKs".parse::<HandClass>().unwrap().to_string(), "AKs");
        assert!("ka".parse::<HandClass>().is_err(), "non-pair needs s/o");
        assert_eq!("KAo".parse::<HandClass>().unwrap().to_string(), "AKo");
        assert!("AAs".parse::<HandClass>().is_err());
        assert!("A".parse::<HandClass>().is_err());
    }
}
