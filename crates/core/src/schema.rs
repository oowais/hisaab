//! Schema migrations. Append new migrations; never edit shipped ones.
//!
//! Sync-ready rules: UUIDv7 text keys, `updated_at` on every synced row,
//! soft deletes (`deleted_at`), instants as UTC epoch milliseconds.

use rusqlite_migration::{Migrations, M};

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(
        "
        -- main_currency, default_remind_days, … (JSON values).
        CREATE TABLE setting (
          key        TEXT PRIMARY KEY,
          value      TEXT NOT NULL,
          updated_at INTEGER NOT NULL
        );
        ",
    )])
}
