//! RiverLab poker domain model.
//!
//! Units: all chip amounts are integer milli-big-blinds ([`Chips`]); see ADR-0002.
//! This crate has no I/O and no dependency on any presentation or solver layer.

pub mod card;
pub mod cardset;
pub mod chips;
pub mod config;
pub mod deck;
pub mod error;
pub mod game;
pub mod holding;
pub mod position;
pub mod rake;
pub mod stack;

pub use card::{Card, Rank, Suit};
pub use cardset::CardSet;
pub use chips::Chips;
pub use config::{GameConfig, Stake};
pub use deck::Deck;
pub use error::PokerError;
pub use holding::{Board, HoleCards, Street};
pub use position::Position;
pub use rake::{RakeConfig, RakeResult};
pub use stack::{StackConfig, StackPreset};

/// Engine version recorded with every persisted computation result.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
