use crate::class::{HandClass, HandKind};
use crate::combo::{COMBO_COUNT, Combo};
use poker_core::CardSet;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum RangeError {
    #[error("invalid range token '{0}'")]
    InvalidToken(String),
    #[error("invalid weight '{0}': weights must be finite numbers in [0, 1]")]
    InvalidWeight(String),
    #[error("invalid range: {0}")]
    Invalid(String),
}

/// A weighted range over the 1,326 combos. Weight = frequency in `[0, 1]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Range {
    weights: Vec<f64>,
}

impl Default for Range {
    fn default() -> Self {
        Range::empty()
    }
}

/// One cell of the 13×13 matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatrixCell {
    pub label: String,
    pub row: usize,
    pub col: usize,
    pub kind: HandKind,
    /// Combos in the class (6/4/12).
    pub combos: usize,
    /// Combos not blocked by the dead cards.
    pub live_combos: usize,
    /// Sum of weights over live combos.
    pub weight_sum: f64,
}

pub(crate) fn check_weight(w: f64) -> Result<f64, RangeError> {
    if w.is_finite() && (0.0..=1.0).contains(&w) { Ok(w) } else { Err(RangeError::InvalidWeight(w.to_string())) }
}

impl Range {
    pub fn empty() -> Range {
        Range { weights: vec![0.0; COMBO_COUNT] }
    }

    /// Every combo at weight 1.
    pub fn full() -> Range {
        Range { weights: vec![1.0; COMBO_COUNT] }
    }

    /// Builds a range from 1,326 weights in combo-index order.
    pub fn from_weights(weights: Vec<f64>) -> Result<Range, RangeError> {
        if weights.len() != COMBO_COUNT {
            return Err(RangeError::Invalid(format!("expected {COMBO_COUNT} weights, got {}", weights.len())));
        }
        for &w in &weights {
            check_weight(w)?;
        }
        Ok(Range { weights })
    }

    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    pub fn weight(&self, c: Combo) -> f64 {
        self.weights[c.index()]
    }

    pub fn set_weight(&mut self, c: Combo, w: f64) -> Result<(), RangeError> {
        self.weights[c.index()] = check_weight(w)?;
        Ok(())
    }

    pub fn set_class(&mut self, class: HandClass, w: f64) -> Result<(), RangeError> {
        let w = check_weight(w)?;
        for c in class.combos() {
            self.weights[c.index()] = w;
        }
        Ok(())
    }

    /// Combos with non-zero weight.
    pub fn iter(&self) -> impl Iterator<Item = (Combo, f64)> + '_ {
        self.weights
            .iter()
            .enumerate()
            .filter(|(_, w)| **w > 0.0)
            .map(|(i, w)| (Combo::from_index(i).expect("<1326"), *w))
    }

    pub fn is_empty(&self) -> bool {
        self.weights.iter().all(|&w| w == 0.0)
    }

    /// Weighted number of combos.
    pub fn combo_count(&self) -> f64 {
        self.weights.iter().sum()
    }

    /// Weighted fraction of all 1,326 starting hands.
    pub fn fraction_of_all_hands(&self) -> f64 {
        self.combo_count() / COMBO_COUNT as f64
    }

    /// A copy with every combo that uses a dead card removed (card removal).
    pub fn without_cards(&self, dead: CardSet) -> Range {
        let mut r = self.clone();
        for c in Combo::all() {
            if c.card_set().intersects(dead) {
                r.weights[c.index()] = 0.0;
            }
        }
        r
    }

    /// Weighted combos still possible given the dead cards.
    pub fn live_combo_count(&self, dead: CardSet) -> f64 {
        self.iter().filter(|(c, _)| !c.card_set().intersects(dead)).map(|(_, w)| w).sum()
    }

    /// Fraction of this range's weight removed by `blockers` (0 when the range is empty).
    pub fn blocked_fraction(&self, blockers: CardSet) -> f64 {
        let total = self.combo_count();
        if total == 0.0 { 0.0 } else { 1.0 - self.live_combo_count(blockers) / total }
    }

    /// The 169-cell matrix view, accounting for dead cards.
    pub fn matrix(&self, dead: CardSet) -> Vec<MatrixCell> {
        HandClass::all()
            .into_iter()
            .map(|class| {
                let (row, col) = class.grid_position();
                let live: Vec<Combo> = class.combos().into_iter().filter(|c| !c.card_set().intersects(dead)).collect();
                MatrixCell {
                    label: class.to_string(),
                    row,
                    col,
                    kind: class.kind(),
                    combos: class.combo_count(),
                    live_combos: live.len(),
                    weight_sum: live.iter().map(|c| self.weight(*c)).sum(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poker_core::Card;

    fn dead(s: &str) -> CardSet {
        CardSet::from_cards(Card::parse_many(s).unwrap())
    }

    #[test]
    fn full_range_counts() {
        assert_eq!(Range::full().combo_count(), 1326.0);
        assert_eq!(Range::full().fraction_of_all_hands(), 1.0);
        // One dead card removes 51 combos; two remove 101.
        assert_eq!(Range::full().live_combo_count(dead("As")), 1275.0);
        assert_eq!(Range::full().live_combo_count(dead("AsKd")), 1225.0);
    }

    #[test]
    fn card_removal_on_classes() {
        let r = Range::parse("AA,AKs").unwrap();
        assert_eq!(r.combo_count(), 10.0);
        // Ace of spades on board: AA 6→3, AKs 4→3.
        assert_eq!(r.live_combo_count(dead("As")), 6.0);
        assert!((r.blocked_fraction(dead("As")) - 0.4).abs() < 1e-12);
        assert_eq!(r.without_cards(dead("As")).combo_count(), 6.0);
    }

    #[test]
    fn weights_are_validated() {
        let mut r = Range::empty();
        let c = Combo::all().next().unwrap();
        assert!(r.set_weight(c, 1.5).is_err());
        assert!(r.set_weight(c, f64::NAN).is_err());
        assert!(r.set_weight(c, -0.1).is_err());
        r.set_weight(c, 0.25).unwrap();
        assert_eq!(r.combo_count(), 0.25);
        assert!(Range::from_weights(vec![0.0; 3]).is_err());
    }

    #[test]
    fn matrix_reflects_weights_and_dead_cards() {
        let r = Range::parse("AKs:0.5").unwrap();
        let m = r.matrix(dead("Ks"));
        let cell = m.iter().find(|c| c.label == "AKs").unwrap();
        assert_eq!((cell.row, cell.col, cell.combos, cell.live_combos), (0, 1, 4, 3));
        assert!((cell.weight_sum - 1.5).abs() < 1e-12);
        assert_eq!(m.len(), 169);
    }
}
