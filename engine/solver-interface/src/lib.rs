//! Solver contract for RiverLab (ADR-0005).
//!
//! **This crate contains no solver.** It defines:
//! - the extensive-form game abstraction a solver traverses ([`ExtensiveGame`]),
//! - the solver, strategy and best-response traits a solver must implement,
//! - validated, versioned configuration ([`SolverConfig`], [`SolveSpec`]) persisted with
//!   every job and result,
//! - two reference formulas that pin down the update rules precisely
//!   ([`regret_matching`], [`DiscountWeights`]). They are building blocks, not a solver.

mod config;
mod traits;

pub use config::*;
pub use traits::*;

/// Regret matching: the current strategy is proportional to positive cumulative regrets,
/// or uniform if no regret is positive. Returns an error for non-finite input.
pub fn regret_matching(regrets: &[f64]) -> Result<Vec<f64>, SolverError> {
    if regrets.is_empty() {
        return Err(SolverError::InvalidInput("no actions".into()));
    }
    if regrets.iter().any(|r| !r.is_finite()) {
        return Err(SolverError::InvalidInput("non-finite regret".into()));
    }
    let pos: f64 = regrets.iter().map(|r| r.max(0.0)).sum();
    Ok(if pos > 0.0 {
        regrets.iter().map(|r| r.max(0.0) / pos).collect()
    } else {
        vec![1.0 / regrets.len() as f64; regrets.len()]
    })
}

/// Per-iteration discount factors of Discounted CFR (Brown & Sandholm, AAAI 2019):
/// after iteration `t`, positive cumulative regrets are multiplied by `t^α / (t^α + 1)`,
/// negative ones by `t^β / (t^β + 1)`, and the average-strategy contribution by
/// `(t / (t + 1))^γ`. CFR+ corresponds to α = ∞, β = −∞ (regret floor at 0), γ = 1
/// (linear averaging) and is handled separately by implementations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscountWeights {
    pub positive_regret: f64,
    pub negative_regret: f64,
    pub strategy: f64,
}

impl DiscountWeights {
    pub fn dcfr(alpha: f64, beta: f64, gamma: f64, t: u64) -> Result<DiscountWeights, SolverError> {
        if t == 0 {
            return Err(SolverError::InvalidInput("iterations are numbered from 1".into()));
        }
        let t = t as f64;
        let f = |e: f64| {
            let p = t.powf(e);
            p / (p + 1.0)
        };
        Ok(DiscountWeights {
            positive_regret: f(alpha),
            negative_regret: f(beta),
            strategy: (t / (t + 1.0)).powf(gamma),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regret_matching_normalises_positive_regret() {
        assert_eq!(regret_matching(&[1.0, 3.0, -2.0]).unwrap(), vec![0.25, 0.75, 0.0]);
        assert_eq!(regret_matching(&[-1.0, 0.0]).unwrap(), vec![0.5, 0.5]);
        assert!(regret_matching(&[]).is_err());
        assert!(regret_matching(&[f64::NAN]).is_err());
    }

    #[test]
    fn dcfr_weights_match_paper_definition() {
        // α = 1.5, β = 0, γ = 2 at t = 1: 1/2, 1/2, 1/4
        let w = DiscountWeights::dcfr(1.5, 0.0, 2.0, 1).unwrap();
        assert_eq!((w.positive_regret, w.negative_regret, w.strategy), (0.5, 0.5, 0.25));
        let w = DiscountWeights::dcfr(1.5, 0.0, 2.0, 4).unwrap();
        assert!((w.positive_regret - 8.0 / 9.0).abs() < 1e-12);
        assert!((w.strategy - 0.64).abs() < 1e-12);
        assert!(DiscountWeights::dcfr(1.5, 0.0, 2.0, 0).is_err());
    }
}
