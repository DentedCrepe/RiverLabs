use crate::traits::SolverError;
use poker_core::{Board, GameConfig};
use serde::{Deserialize, Serialize};

pub const SOLVER_CONFIG_SCHEMA_VERSION: u32 = 1;

/// Monte Carlo CFR sampling scheme.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "scheme", rename_all = "snake_case", deny_unknown_fields)]
pub enum SamplingScheme {
    External,
    /// Outcome sampling with ε-exploration in (0, 1].
    Outcome {
        exploration: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "algorithm", rename_all = "snake_case", deny_unknown_fields)]
pub enum CfrVariant {
    /// Vanilla CFR (Zinkevich et al. 2007).
    Vanilla,
    /// CFR+ (Tammelin 2014): regrets floored at 0, alternating updates, linear averaging.
    CfrPlus,
    /// Discounted CFR (Brown & Sandholm 2019). Paper's recommended α=1.5, β=0, γ=2.
    Discounted { alpha: f64, beta: f64, gamma: f64 },
    /// Monte Carlo CFR (Lanctot et al. 2009).
    MonteCarlo { sampling: SamplingScheme },
}

impl CfrVariant {
    pub fn dcfr_default() -> CfrVariant {
        CfrVariant::Discounted { alpha: 1.5, beta: 0.0, gamma: 2.0 }
    }
    pub fn is_sampling(&self) -> bool {
        matches!(self, CfrVariant::MonteCarlo { .. })
    }
}

/// How iterations are weighted in the average strategy (DCFR uses its own γ).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AveragingScheme {
    Uniform,
    Linear,
    Quadratic,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoppingCriteria {
    /// Hard upper bound; always required so no job runs unbounded.
    pub max_iterations: u64,
    /// Stop once exploitability (as a percentage of the starting pot) is at or below this.
    pub target_exploitability_pct_pot: Option<f64>,
    pub max_seconds: Option<u64>,
    /// Measure exploitability every N iterations (best response is expensive).
    pub exploitability_check_interval: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverConfig {
    pub schema_version: u32,
    pub variant: CfrVariant,
    pub averaging: AveragingScheme,
    pub stopping: StoppingCriteria,
    pub threads: u16,
    /// Required for sampling variants so results are reproducible.
    pub seed: Option<u64>,
}

impl Default for SolverConfig {
    fn default() -> Self {
        SolverConfig {
            schema_version: SOLVER_CONFIG_SCHEMA_VERSION,
            variant: CfrVariant::dcfr_default(),
            averaging: AveragingScheme::Quadratic,
            stopping: StoppingCriteria {
                max_iterations: 1000,
                target_exploitability_pct_pot: Some(0.5),
                max_seconds: None,
                exploitability_check_interval: 10,
            },
            threads: 1,
            seed: None,
        }
    }
}

fn invalid<T>(msg: impl Into<String>) -> Result<T, SolverError> {
    Err(SolverError::InvalidConfig(msg.into()))
}

impl SolverConfig {
    pub fn validate(&self) -> Result<(), SolverError> {
        if self.schema_version != SOLVER_CONFIG_SCHEMA_VERSION {
            return invalid(format!("unsupported schema version {}", self.schema_version));
        }
        match self.variant {
            CfrVariant::Discounted { alpha, beta, gamma } => {
                if ![alpha, beta, gamma].iter().all(|x| x.is_finite()) {
                    return invalid("DCFR parameters must be finite");
                }
                if alpha < 0.0 || gamma < 0.0 {
                    return invalid("DCFR alpha and gamma must be non-negative");
                }
            }
            CfrVariant::MonteCarlo { sampling } => {
                if let SamplingScheme::Outcome { exploration } = sampling {
                    if !(exploration > 0.0 && exploration <= 1.0) {
                        return invalid("outcome-sampling exploration must be in (0, 1]");
                    }
                }
                if self.seed.is_none() {
                    return invalid("Monte Carlo CFR requires a seed for reproducibility");
                }
            }
            CfrVariant::Vanilla | CfrVariant::CfrPlus => {}
        }
        let s = &self.stopping;
        if s.max_iterations == 0 {
            return invalid("max_iterations must be positive");
        }
        if s.exploitability_check_interval == 0 {
            return invalid("exploitability_check_interval must be positive");
        }
        if let Some(t) = s.target_exploitability_pct_pot {
            if !(t.is_finite() && t > 0.0) {
                return invalid("target exploitability must be a positive percentage");
            }
        }
        if self.threads == 0 || self.threads > 256 {
            return invalid("threads must be between 1 and 256");
        }
        Ok(())
    }
}

/// Bet-size abstraction for building a hold'em game tree (used from Milestone 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeAbstraction {
    /// Bet sizes as percentages of the pot, per postflop street (flop, turn, river).
    pub bet_sizes_pct_pot: [Vec<u32>; 3],
    /// Raise sizes as percentages of the pot after calling, per street.
    pub raise_sizes_pct_pot: [Vec<u32>; 3],
    pub max_raises_per_street: u8,
    /// Bets leaving less than this percentage of the pot behind become all-in.
    pub all_in_threshold_pct_pot: u32,
}

impl TreeAbstraction {
    pub fn validate(&self) -> Result<(), SolverError> {
        for street in self.bet_sizes_pct_pot.iter().chain(&self.raise_sizes_pct_pot) {
            if street.iter().any(|&p| p == 0 || p > 10_000) {
                return invalid("bet and raise sizes must be between 1% and 10000% of the pot");
            }
        }
        if self.max_raises_per_street > 10 {
            return invalid("max_raises_per_street must be at most 10");
        }
        Ok(())
    }
}

/// Everything that defines a postflop solve. Persisted verbatim with every job and result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolveSpec {
    pub game: GameConfig,
    /// Range notation per player in postflop action order (OOP first).
    pub ranges: Vec<String>,
    pub board: Board,
    /// Pot and effective stack at the start of the subgame, in mbb.
    pub starting_pot_mbb: u64,
    pub effective_stack_mbb: u64,
    pub tree: TreeAbstraction,
    pub solver: SolverConfig,
}

impl SolveSpec {
    pub fn validate(&self) -> Result<(), SolverError> {
        self.game.validate().map_err(|e| SolverError::InvalidConfig(e.to_string()))?;
        if self.ranges.len() != 2 {
            return invalid("postflop solves are heads-up: exactly two ranges are required");
        }
        if self.board.cards().len() < 3 {
            return invalid("postflop solves need a flop, turn or river board");
        }
        if self.starting_pot_mbb == 0 || self.effective_stack_mbb == 0 {
            return invalid("starting pot and effective stack must be positive");
        }
        self.tree.validate()?;
        self.solver.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid_dcfr() {
        let c = SolverConfig::default();
        c.validate().unwrap();
        assert_eq!(c.variant, CfrVariant::Discounted { alpha: 1.5, beta: 0.0, gamma: 2.0 });
    }

    #[test]
    fn validation_rejects_bad_parameters() {
        let base = SolverConfig::default();
        let mut c = base.clone();
        c.variant = CfrVariant::MonteCarlo { sampling: SamplingScheme::External };
        assert!(c.validate().is_err(), "MCCFR without a seed");
        c.seed = Some(1);
        c.validate().unwrap();
        c.variant = CfrVariant::MonteCarlo { sampling: SamplingScheme::Outcome { exploration: 0.0 } };
        assert!(c.validate().is_err());
        let mut c = base.clone();
        c.stopping.max_iterations = 0;
        assert!(c.validate().is_err());
        let mut c = base.clone();
        c.variant = CfrVariant::Discounted { alpha: f64::NAN, beta: 0.0, gamma: 2.0 };
        assert!(c.validate().is_err());
        let mut c = base;
        c.threads = 0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn serde_shape_is_stable() {
        let json = serde_json::to_value(SolverConfig::default()).unwrap();
        assert_eq!(json["variant"]["algorithm"], "discounted");
        assert_eq!(json["averaging"], "quadratic");
        let back: SolverConfig = serde_json::from_value(json).unwrap();
        assert_eq!(back, SolverConfig::default());
        assert!(serde_json::from_str::<SolverConfig>(r#"{"bogus":1}"#).is_err());
    }
}
