use crate::chips::Chips;
use crate::holding::Street;
use crate::position::Position;
use serde::{Deserialize, Serialize};

/// A player decision. Bet and raise amounts are "to" amounts: the player's total commitment
/// on the current street after the action. All-ins are ordinary bets/raises/calls whose amount
/// uses the whole stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Fold,
    Check,
    Call,
    Bet { to: Chips },
    Raise { to: Chips },
}

/// One entry of the betting history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub seat: usize,
    pub position: Position,
    pub street: Street,
    pub action: Action,
    /// Chips moved from the stack into the pot by this action.
    pub added: Chips,
    /// Street commitment after the action.
    pub street_commit: Chips,
    pub all_in: bool,
    /// True for bets/raises that are at least a full (minimum) raise.
    pub full_raise: bool,
}

/// Legal options for the player to act. `bet`/`raise` give inclusive `(min_to, max_to)`
/// bounds; amounts strictly between a short all-in and the minimum are not legal — when
/// `min_to == max_to` the only option is that exact amount (usually all-in).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalActions {
    pub seat: usize,
    pub fold: bool,
    pub check: bool,
    /// Chips added by calling (capped at the stack).
    pub call: Option<Chips>,
    pub bet: Option<(Chips, Chips)>,
    pub raise: Option<(Chips, Chips)>,
}
