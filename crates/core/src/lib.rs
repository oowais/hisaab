//! Hisaab core: storage, occurrence engine and currency logic.
//!
//! All writes go through [`Store`]; triggers log them for sync (`sync.rs`). No Tauri or platform code here: the app calls it
//! from Tauri commands, Android calls it via JNI when no webview runs.

mod conflicts;
pub mod currency;
pub mod dates;
pub mod invariants;
mod model;
pub mod occurrence;
mod overview;
mod peers;
pub mod reminders;
mod schema;
mod store;
mod subscriptions;
mod sync;
mod fx;

pub use conflicts::Conflict;
pub use model::*;
pub use overview::{per_year, Detail, HistoryCharge, Overview, SubscriptionView, TimelineLane, UpcomingCharge};
pub use reminders::{snooze_picks, Fired, Reminder, SnoozePick};
pub use store::{Settings, Store};
pub use peers::{Peer, SyncMode, SyncSettings};
pub use subscriptions::{NewOverride, NewSubscription};
pub use sync::{MergeOutcome, SyncChange};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("migration: {0}")]
    Migration(String),
}

pub type Result<T> = std::result::Result<T, Error>;
