//! Play against bots at custom tables (cash, fast-fold, sit & go), then review a session report.
//!
//! - [`Session`] runs the game: dealing (seeded), the betting engine from `poker-core`, bots,
//!   blind levels and eliminations.
//! - Bots ([`bot`]) use GTOpen's preflop model (approximate) and transparent postflop rules.
//! - [`report::build`] turns hand records into statistics and findings, each stating what it
//!   was measured against.
//!
//! See `docs/play.md`.

pub mod bot;
pub mod config;
pub mod drill;
pub mod judge;
pub mod model;
pub mod preflop_drill;
pub mod push_fold;
pub mod record;
pub mod report;
pub mod review;
pub mod session;
pub mod spot;
pub mod strength;
pub mod view;

pub use config::{Difficulty, Format, Level, PlayError, TableConfig};
pub use model::ModelCache;
pub use record::HandRecord;
pub use report::Report;
pub use session::{Session, SessionStatus};
