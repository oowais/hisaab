//! Schema migrations. Append new migrations; never edit shipped ones.
//!
//! Sync-ready rules: UUIDv7 text keys, `updated_at` on every synced row,
//! soft deletes (`deleted_at`), instants as UTC epoch milliseconds.

use rusqlite_migration::{Migrations, M};

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(
            "
        -- main_currency, default_remind_days, … (JSON values).
        CREATE TABLE setting (
          key        TEXT PRIMARY KEY,
          value      TEXT NOT NULL,
          updated_at INTEGER NOT NULL
        );
        ",
        ),
        // Domain tables. Dates are calendar days 'YYYY-MM-DD'; money is
        // integer minor units. No foreign keys: sync may deliver a row
        // before the one it points to.
        M::up(
            "
        CREATE TABLE category (
          id         TEXT PRIMARY KEY,
          name       TEXT NOT NULL,
          color      TEXT NOT NULL DEFAULT '',
          sort       INTEGER NOT NULL DEFAULT 0,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL,
          deleted_at INTEGER
        );

        CREATE TABLE payment_method (
          id         TEXT PRIMARY KEY,
          name       TEXT NOT NULL,             -- 'Visa ..4417', 'PayPal'
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL,
          deleted_at INTEGER
        );

        -- Identity; never changes with price.
        CREATE TABLE subscription (
          id                 TEXT PRIMARY KEY,
          kind               TEXT NOT NULL CHECK (kind IN ('subscription', 'bill')),
          name               TEXT NOT NULL,
          template_key       TEXT,              -- e.g. 'netflix'
          icon               TEXT,
          color              TEXT,
          category_id        TEXT,
          payment_method_id  TEXT,
          url                TEXT,
          notes              TEXT NOT NULL DEFAULT '',
          remind_days_before INTEGER,           -- NULL = global default
          archived_at        INTEGER,
          created_at         INTEGER NOT NULL,
          updated_at         INTEGER NOT NULL,
          deleted_at         INTEGER
        );

        -- Contiguous stretch with one set of terms; never overlapping
        -- within a subscription (checked on save, invariants.rs).
        CREATE TABLE period (
          id              TEXT PRIMARY KEY,
          subscription_id TEXT NOT NULL,
          status          TEXT NOT NULL CHECK (status IN ('trial', 'active', 'paused')),
          plan_label      TEXT,
          amount_minor    INTEGER NOT NULL,
          currency        TEXT NOT NULL,
          every_n         INTEGER NOT NULL CHECK (every_n >= 1),
          unit            TEXT NOT NULL CHECK (unit IN ('day', 'week', 'month', 'year')),
          anchor_date     TEXT NOT NULL,        -- first charge date
          start_date      TEXT NOT NULL,
          end_date        TEXT,                 -- NULL = open-ended
          amount_kind     TEXT NOT NULL DEFAULT 'fixed' CHECK (amount_kind IN ('fixed', 'estimated')),
          created_at      INTEGER NOT NULL,
          updated_at      INTEGER NOT NULL,
          deleted_at      INTEGER
        );
        CREATE INDEX period_subscription ON period(subscription_id);

        -- Only when reality differs from the rule.
        CREATE TABLE charge_override (
          id                   TEXT PRIMARY KEY,
          period_id            TEXT NOT NULL,
          due_date             TEXT NOT NULL,   -- the scheduled date
          kind                 TEXT NOT NULL CHECK (kind IN ('skipped', 'moved', 'amount_changed', 'settled')),
          new_due_date         TEXT,
          actual_amount_minor  INTEGER,
          actual_currency      TEXT,
          settled_amount_minor INTEGER,         -- what the bank actually took
          settled_currency     TEXT,
          created_at           INTEGER NOT NULL,
          updated_at           INTEGER NOT NULL,
          deleted_at           INTEGER
        );
        CREATE INDEX charge_override_period ON charge_override(period_id, due_date);

        -- Local cache of ECB rates (units per 1 EUR). Not synced.
        CREATE TABLE fx_rate (
          date        TEXT NOT NULL,
          currency    TEXT NOT NULL,
          rate_vs_eur REAL NOT NULL,
          PRIMARY KEY (date, currency)
        );
        ",
        ),
    ])
}
