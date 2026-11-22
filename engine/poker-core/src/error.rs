use thiserror::Error;

/// Errors produced by the poker domain model. Every invalid input is reported; nothing is
/// silently corrected.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PokerError {
    #[error("invalid card '{0}': expected rank (2-9,T,J,Q,K,A) followed by suit (c,d,h,s)")]
    InvalidCard(String),
    #[error("invalid rank '{0}'")]
    InvalidRank(char),
    #[error("invalid suit '{0}'")]
    InvalidSuit(char),
    #[error("duplicate card {0}")]
    DuplicateCard(String),
    #[error("invalid board: {0}")]
    InvalidBoard(String),
    #[error("invalid hole cards: {0}")]
    InvalidHoleCards(String),
    #[error("card {0} is not in the deck")]
    CardNotInDeck(String),
    #[error("deck exhausted")]
    DeckExhausted,
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("illegal action: {0}")]
    IllegalAction(String),
    #[error("invalid game state transition: {0}")]
    InvalidTransition(String),
    #[error("arithmetic overflow")]
    Overflow,
}

pub type Result<T> = std::result::Result<T, PokerError>;
