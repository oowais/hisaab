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
        // Reminder bookkeeping for this device (not synced): what already
        // fired, and a snoozed reminder.
        M::up(
            "
        CREATE TABLE reminder_state (
          subscription_id TEXT PRIMARY KEY,
          fired_through   INTEGER NOT NULL,     -- UTC ms; reminders at or before it are done
          snooze_at       INTEGER,              -- UTC ms
          snooze_title    TEXT,
          snooze_text     TEXT
        );
        ",
        ),
        // P2P sync (copied from Befikr). Device-local bookkeeping: none of
        // these tables sync.
        M::up(
            "
        -- One row: this device's id, its hybrid logical clock, whether
        -- remote changes are being applied (then triggers don't log), and
        -- the sync settings.
        CREATE TABLE sync_state (
          id         INTEGER PRIMARY KEY CHECK (id = 1),
          device     TEXT NOT NULL,
          hlc        INTEGER NOT NULL DEFAULT 0,
          applying   INTEGER NOT NULL DEFAULT 0,
          backfilled INTEGER NOT NULL DEFAULT 0,
          mode       TEXT NOT NULL DEFAULT 'off',   -- off|relay
          name       TEXT NOT NULL DEFAULT ''       -- shown to other devices
        );
        INSERT INTO sync_state (id, device) VALUES (1, lower(hex(randomblob(16))));

        -- Latest value per synced field, filled by triggers (sync.rs).
        -- `seq` grows on every change: peers ask for everything after the
        -- last seq they got from us.
        CREATE TABLE changes (
          seq      INTEGER PRIMARY KEY AUTOINCREMENT,
          tbl      TEXT NOT NULL,
          row_id   TEXT NOT NULL,
          field    TEXT NOT NULL,
          value,                          -- any type, as stored in the row
          hlc      INTEGER NOT NULL,
          origin   TEXT NOT NULL,         -- device that made the change
          prev_hlc INTEGER,               -- hlc of the version it replaced (NULL = new)
          UNIQUE (tbl, row_id, field)
        );
        -- Scratch for the change-log triggers (always emptied first).
        CREATE TABLE change_prev (field TEXT PRIMARY KEY, hlc INTEGER);

        CREATE TABLE peers (
          endpoint     TEXT PRIMARY KEY,  -- iroh endpoint id (public key)
          device       TEXT NOT NULL,     -- their sync_state.device
          name         TEXT NOT NULL,
          their_seq    INTEGER NOT NULL DEFAULT 0,  -- last of their changes merged here
          paired_at    INTEGER NOT NULL,
          last_sync_at INTEGER,
          addrs        TEXT NOT NULL DEFAULT '[]',  -- last known \"ip:port\" list
          kind         TEXT NOT NULL DEFAULT '',    -- 'phone' | 'computer'
          updated_at   INTEGER NOT NULL,
          deleted_at   INTEGER
        );

        -- This device's edits that lost to a concurrent edit from another
        -- device, and periods that overlap after a merge (field 'overlap':
        -- lost_value = our period id, won_value = theirs).
        CREATE TABLE conflicts (
          id          TEXT PRIMARY KEY,   -- UUIDv7
          tbl         TEXT NOT NULL,
          row_id      TEXT NOT NULL,
          field       TEXT NOT NULL,
          lost_value,
          won_value,
          won_origin  TEXT,               -- device whose edit won
          at          INTEGER NOT NULL,   -- when it was noticed
          resolution  TEXT,               -- 'mine' | 'theirs' once reviewed
          updated_at  INTEGER NOT NULL,
          deleted_at  INTEGER
        );
        CREATE INDEX conflicts_open ON conflicts(resolution, at);
        ",
        ),
        // A shared payment: the percent that's yours (NULL = all of it).
        M::up("ALTER TABLE subscription ADD COLUMN my_share INTEGER;"),
        // One-time charges on a subscription, besides its schedule.
        M::up(
            "CREATE TABLE extra_charge (
              id              TEXT PRIMARY KEY,
              subscription_id TEXT NOT NULL,
              due_date        TEXT NOT NULL,
              amount_minor    INTEGER NOT NULL,
              currency        TEXT NOT NULL,
              label           TEXT NOT NULL DEFAULT '',
              created_at      INTEGER NOT NULL,
              updated_at      INTEGER NOT NULL,
              deleted_at      INTEGER
            );
            CREATE INDEX extra_charge_sub ON extra_charge(subscription_id, due_date);",
        ),
        // This device's own settings (not synced, kept on restore): e.g.
        // reminders on or off here.
        M::up(
            "CREATE TABLE local_setting (
              key   TEXT PRIMARY KEY,
              value TEXT NOT NULL      -- JSON
            );",
        ),
    ])
}
