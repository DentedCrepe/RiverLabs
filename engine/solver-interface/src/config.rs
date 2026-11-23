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

/// One player's bet sizes per postflop street (`[flop, turn, river]`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerSizing {
    /// Bets as percentages of the pot.
    pub bet_pct_pot: [Vec<u32>; 3],
    /// Raises as percentages of the pot after calling.
    pub raise_pct_pot: [Vec<u32>; 3],
}

/// Bet-size abstraction for building a hold'em game tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeAbstraction {
    pub oop: PlayerSizing,
    pub ip: PlayerSizing,
    pub max_raises_per_street: u8,
    /// A bet or raise that would commit at least this percentage of the player's stack
    /// (including chips already bet on the street) becomes all-in. 100 = only exact all-ins.
    pub all_in_threshold_pct_stack: u32,
    /// Always offer an all-in in addition to the configured sizes.
    pub add_all_in: bool,
}

impl TreeAbstraction {
    pub fn validate(&self) -> Result<(), SolverError> {
        for p in [&self.oop, &self.ip] {
            for street in p.bet_pct_pot.iter().chain(&p.raise_pct_pot) {
                if street.iter().any(|&x| x == 0 || x > 10_000) {
                    return invalid("bet and raise sizes must be between 1% and 10000% of the pot");
                }
            }
        }
        if self.max_raises_per_street > 10 {
            return invalid("max_raises_per_street must be at most 10");
        }
        if !(1..=100).contains(&self.all_in_threshold_pct_stack) {
            return invalid("all_in_threshold_pct_stack must be between 1 and 100");
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

/// How a preflop model values the pots that reach the flop. Preflop solving in RiverLab is an
/// **approximate model**: the flop is not played out (see `docs/solver-validation.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquityRealization {
    /// Every player realises exactly their all-in equity.
    Full,
    /// A fixed positional adjustment: the last player to act postflop realises more of their
    /// equity, the first less, growing with stack-to-pot ratio (GTOpen's "static" model).
    Positional,
}

/// Preflop bet-size abstraction. Sizes are "raise to" amounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreflopSizing {
    /// First-raise sizes, as the total raised to, in mbb.
    pub open_raise_to_mbb: Vec<u64>,
    /// Re-raise sizes as a percentage of the bet faced (300 = raise to 3x).
    pub reraise_pct_of_bet: Vec<u32>,
    /// Raises allowed in total, the open included (2 = open and one 3-bet).
    pub max_raises: u8,
    /// Always offer an all-in in addition to the configured sizes.
    pub add_all_in: bool,
    /// A raise to more than this percentage of the stack becomes all-in.
    pub all_in_threshold_pct_stack: u32,
    /// Allow limping (calling the big blind without a raise).
    pub allow_limp: bool,
}

/// Everything that defines a preflop solve. Persisted verbatim with every job and result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreflopSpec {
    /// Players, blinds, ante, stack and rake. `heads_up` overrides the player count.
    pub game: GameConfig,
    /// Small blind against big blind only.
    pub heads_up: bool,
    pub sizing: PreflopSizing,
    pub realization: EquityRealization,
    pub max_iterations: u64,
    pub max_seconds: Option<u64>,
    /// Measure each player's best-response gap every N iterations.
    pub check_interval: u64,
    pub threads: u16,
}

impl PreflopSpec {
    pub fn players(&self) -> usize {
        if self.heads_up { 2 } else { self.game.players as usize }
    }

    pub fn validate(&self) -> Result<(), SolverError> {
        self.game.validate().map_err(|e| SolverError::InvalidConfig(e.to_string()))?;
        let s = &self.sizing;
        let stack = self.game.stack.effective_stack.mbb();
        let bb = GameConfig::BIG_BLIND.mbb();
        if s.open_raise_to_mbb.is_empty() && !s.add_all_in {
            return invalid("give at least one open-raise size or allow all-in");
        }
        if s.open_raise_to_mbb.len() > 4 || s.reraise_pct_of_bet.len() > 4 {
            return invalid("at most 4 open sizes and 4 re-raise sizes");
        }
        if s.open_raise_to_mbb.iter().any(|&o| o < 2 * bb || o >= stack) {
            return invalid("open-raise sizes must be at least 2bb and less than the stack");
        }
        if s.reraise_pct_of_bet.iter().any(|&m| !(200..=1000).contains(&m)) {
            return invalid("re-raise sizes must be between 2x (200) and 10x (1000) the bet faced");
        }
        if !(1..=6).contains(&s.max_raises) {
            return invalid("max_raises must be between 1 and 6");
        }
        if !(1..=100).contains(&s.all_in_threshold_pct_stack) {
            return invalid("all_in_threshold_pct_stack must be between 1 and 100");
        }
        if self.max_iterations == 0 || self.check_interval == 0 {
            return invalid("max_iterations and check_interval must be positive");
        }
        if self.max_iterations > u32::MAX as u64 {
            return invalid("max_iterations must be at most 4294967295");
        }
        if self.threads == 0 {
            return invalid("threads must be at least 1");
        }
        Ok(())
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

    fn preflop() -> PreflopSpec {
        PreflopSpec {
            game: GameConfig::default(),
            heads_up: false,
            sizing: PreflopSizing {
                open_raise_to_mbb: vec![2_500],
                reraise_pct_of_bet: vec![300],
                max_raises: 3,
                add_all_in: true,
                all_in_threshold_pct_stack: 85,
                allow_limp: false,
            },
            realization: EquityRealization::Positional,
            max_iterations: 100,
            max_seconds: None,
            check_interval: 10,
            threads: 1,
        }
    }

    #[test]
    fn preflop_spec_validation() {
        let base = preflop();
        base.validate().unwrap();
        assert_eq!(base.players(), 6);
        assert_eq!(PreflopSpec { heads_up: true, ..base.clone() }.players(), 2);
        let bad = |f: &dyn Fn(&mut PreflopSpec)| {
            let mut s = base.clone();
            f(&mut s);
            s.validate().is_err()
        };
        assert!(bad(&|s| s.sizing.open_raise_to_mbb = vec![1_500]), "open below a min-raise");
        assert!(bad(&|s| s.sizing.open_raise_to_mbb = vec![100_000]), "open at the stack");
        assert!(bad(&|s| {
            s.sizing.open_raise_to_mbb.clear();
            s.sizing.add_all_in = false;
        }));
        assert!(bad(&|s| s.sizing.reraise_pct_of_bet = vec![150]));
        assert!(bad(&|s| s.sizing.max_raises = 0));
        assert!(bad(&|s| s.sizing.all_in_threshold_pct_stack = 0));
        assert!(bad(&|s| s.max_iterations = 0));
        assert!(bad(&|s| s.threads = 0));
        let json = serde_json::to_value(&base).unwrap();
        assert_eq!(json["realization"], "positional");
        assert_eq!(serde_json::from_value::<PreflopSpec>(json).unwrap(), base);
    }
}
