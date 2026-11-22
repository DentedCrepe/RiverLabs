use crate::config::SolverConfig;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum SolverError {
    #[error("invalid solver configuration: {0}")]
    InvalidConfig(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("solve cancelled")]
    Cancelled,
    #[error("not implemented: {0}")]
    NotImplemented(String),
}

/// What kind of decision point a game state is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NodeKind {
    Terminal,
    Chance,
    Player { player: usize },
}

/// A finite two-or-more-player extensive-form game with imperfect information.
/// Hold'em subgames, Kuhn and Leduc poker all implement this, so solver code can be
/// validated on toy games with known solutions before it touches hold'em.
pub trait ExtensiveGame {
    type State: Clone;
    type Action: Clone + PartialEq + std::fmt::Debug;
    /// Identifies an information set: all states a player cannot distinguish.
    type InfoSet: Clone + Eq + std::hash::Hash + std::fmt::Debug;

    fn num_players(&self) -> usize;
    fn root(&self) -> Self::State;
    fn node_kind(&self, state: &Self::State) -> NodeKind;
    /// Legal actions at a player node, in a fixed deterministic order.
    fn actions(&self, state: &Self::State) -> Vec<Self::Action>;
    /// Outcomes and probabilities at a chance node (probabilities sum to 1).
    fn chance_outcomes(&self, state: &Self::State) -> Vec<(Self::Action, f64)>;
    fn next(&self, state: &Self::State, action: &Self::Action) -> Self::State;
    fn info_set(&self, state: &Self::State, player: usize) -> Self::InfoSet;
    /// Payoff to `player` at a terminal state, in the game's chip unit (mbb for hold'em).
    fn utility(&self, state: &Self::State, player: usize) -> f64;
}

/// A behavioural strategy for every player: action probabilities per information set.
pub trait StrategyProfile<G: ExtensiveGame> {
    /// Probabilities aligned with `G::actions` at any state in the information set, or
    /// `None` if the profile has no entry (callers must treat that as an error, not as
    /// uniform play).
    fn probabilities(&self, info_set: &G::InfoSet) -> Option<Vec<f64>>;
}

/// Progress reported by a running solve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveProgress {
    pub iterations: u64,
    /// Exploitability of the average strategy, if it was measured at this point,
    /// in mbb per hand (hold'em) or game units (toy games).
    pub exploitability: Option<f64>,
    pub elapsed_ms: u64,
}

/// An iterative equilibrium solver (CFR family). Implementations must be deterministic for
/// a given configuration and seed.
pub trait Solver<G: ExtensiveGame> {
    type Profile: StrategyProfile<G>;

    fn new(game: G, config: SolverConfig) -> Result<Self, SolverError>
    where
        Self: Sized;
    /// Runs `n` iterations (or fewer if `should_stop` returns true between iterations).
    fn iterate(&mut self, n: u64, should_stop: &dyn Fn() -> bool) -> Result<SolveProgress, SolverError>;
    /// The average strategy — the quantity that converges to equilibrium.
    fn average_strategy(&self) -> Self::Profile;
    /// The current (last-iterate) strategy.
    fn current_strategy(&self) -> Self::Profile;
    fn iterations(&self) -> u64;
}

/// Best-response computation over a fixed opponent profile.
pub trait BestResponse<G: ExtensiveGame> {
    /// Expected value for `player` of a best response against `profile`.
    fn best_response_value(
        &self,
        game: &G,
        profile: &dyn StrategyProfile<G>,
        player: usize,
    ) -> Result<f64, SolverError>;

    /// Expected value for every player when everyone follows `profile`.
    fn profile_values(&self, game: &G, profile: &dyn StrategyProfile<G>) -> Result<Vec<f64>, SolverError>;

    /// Exploitability for two-player zero-sum games: the average gain available to a
    /// best-responding player, `(BR_0 + BR_1) / 2` measured against the profile's values.
    /// 0 exactly at a Nash equilibrium.
    fn exploitability(&self, game: &G, profile: &dyn StrategyProfile<G>) -> Result<f64, SolverError> {
        if game.num_players() != 2 {
            return Err(SolverError::NotImplemented(
                "exploitability is defined here for 2-player zero-sum games".into(),
            ));
        }
        let v = self.profile_values(game, profile)?;
        let br0 = self.best_response_value(game, profile, 0)?;
        let br1 = self.best_response_value(game, profile, 1)?;
        Ok(((br0 - v[0]) + (br1 - v[1])) / 2.0)
    }
}
