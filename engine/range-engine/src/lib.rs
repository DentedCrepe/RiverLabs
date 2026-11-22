//! Hold'em ranges: the 169 starting-hand classes, the 1,326 two-card combos, weighted
//! (frequency) ranges, range notation, card removal and blocker effects.
//!
//! A [`Range`] is a weight in `[0, 1]` for each of the 1,326 combos. Weights are
//! frequencies supplied by the user or by a validated computation — this crate never
//! invents strategy frequencies.

mod class;
mod combo;
mod notation;
mod range;

pub use class::{HandClass, HandKind};
pub use combo::{COMBO_COUNT, Combo};
pub use range::{MatrixCell, Range, RangeError};
