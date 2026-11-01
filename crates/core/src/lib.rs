//! Hisaab core: storage, occurrence engine and currency logic.
//!
//! All writes go through [`Store`]; it is the single place to emit sync
//! change events later. No Tauri or platform code here: the app calls it
//! from Tauri commands, Android calls it via JNI when no webview runs.

pub mod currency;
pub mod dates;
pub mod invariants;
mod model;
pub mod occurrence;
mod overview;
mod schema;
mod store;
mod subscriptions;
mod fx;

pub use model::*;
pub use overview::{per_year, Detail, HistoryCharge, Overview, SubscriptionView, TimelineLane, UpcomingCharge};
pub use store::{Settings, Store};
pub use subscriptions::{NewOverride, NewSubscription};

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
