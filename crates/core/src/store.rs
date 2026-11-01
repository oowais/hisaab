//! The repository: owns the SQLite connection; every read and write goes
//! through here.

use rusqlite::{params, Connection, OptionalExtension};

use serde::{Deserialize, Serialize};

use crate::{schema, sync, Error, Result};

/// App-wide settings (synced). Theme and the like are per device (frontend).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Totals and converted amounts are shown in it.
    pub main_currency: String,
    /// Reminders on this device (off: none is armed or shown here, snoozed
    /// ones too). Device-local: not synced.
    pub reminders_on: bool,
    /// Remind this many days before a charge, unless a subscription says otherwise.
    pub default_remind_days: u32,
    /// Quick add's currency when no template says otherwise.
    pub last_currency: Option<String>,
    /// Reminders show at this local time, "HH:MM".
    pub reminder_time: String,
    /// What payments should cost a month at most, in the main currency
    /// (minor units). None = no budget.
    pub monthly_budget: Option<i64>,
}

pub struct Store {
    pub(crate) conn: Connection,
    clock: Box<dyn Fn() -> i64 + Send>,
}

pub(crate) fn system_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

impl Store {
    pub fn open(path: &str) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        schema::migrations().to_latest(&mut conn).map_err(|e| Error::Migration(e.to_string()))?;
        sync::install(&conn)?;
        Ok(Self { conn, clock: Box::new(system_now) })
    }

    /// Replace the clock (tests).
    pub fn with_clock(mut self, clock: impl Fn() -> i64 + Send + 'static) -> Self {
        self.clock = Box::new(clock);
        self
    }

    pub fn now(&self) -> i64 {
        (self.clock)()
    }

    pub(crate) fn write<T>(&mut self, f: impl FnOnce(&Connection, i64) -> Result<T>) -> Result<T> {
        let now = self.now();
        let tx = self.conn.transaction()?;
        sync::bump_clock(&tx, now)?;
        let out = f(&tx, now)?;
        tx.commit()?;
        Ok(out)
    }

    /// A setting's JSON value, or None when never set.
    pub fn setting(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT value FROM setting WHERE key = ?1", [key], |r| r.get(0))
            .optional()?;
        raw.map(|v| serde_json::from_str(&v).map_err(|e| Error::Invalid(format!("setting {key}: {e}"))))
            .transpose()
    }

    pub fn set_setting(&mut self, key: &str, value: &serde_json::Value) -> Result<()> {
        if key.is_empty() {
            return Err(Error::Invalid("setting key is empty".into()));
        }
        let value = value.to_string();
        self.write(|c, now| {
            c.execute(
                "INSERT INTO setting (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                params![key, value, now],
            )?;
            Ok(())
        })
    }

    pub fn settings(&self) -> Result<Settings> {
        Ok(Settings {
            main_currency: self.main_currency()?,
            reminders_on: self.local_setting("reminders_on")?.and_then(|v| v.as_bool()).unwrap_or(true),
            default_remind_days: self
                .setting("default_remind_days")?
                .and_then(|v| v.as_u64())
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(3),
            reminder_time: self.reminder_time()?,
            last_currency: self
                .setting("last_currency")?
                .and_then(|v| v.as_str().map(String::from))
                .filter(|c| crate::currency::is_currency_code(c)),
            monthly_budget: self.setting("monthly_budget")?.and_then(|v| v.as_i64()).filter(|n| *n > 0),
        })
    }

    fn local_setting(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT value FROM local_setting WHERE key = ?1", [key], |r| r.get(0))
            .optional()?;
        Ok(raw.and_then(|v| serde_json::from_str(&v).ok()))
    }

    /// "HH:MM", default 09:00.
    pub fn reminder_time(&self) -> Result<String> {
        Ok(self
            .setting("reminder_time")?
            .and_then(|v| v.as_str().map(String::from))
            .filter(|t| chrono::NaiveTime::parse_from_str(t, "%H:%M").is_ok())
            .unwrap_or_else(|| "09:00".into()))
    }

    /// Change one setting, validated (`main_currency`, `default_remind_days`,
    /// `last_currency`, `reminder_time`, `monthly_budget`: minor units of the
    /// main currency, null = none). A new main currency takes the budget
    /// along, converted at the latest rates.
    pub fn update_setting(&mut self, key: &str, value: &serde_json::Value) -> Result<()> {
        if key == "main_currency" {
            if let (Some(budget), Some(to)) = (self.settings()?.monthly_budget, value.as_str()) {
                let from = self.main_currency()?;
                if crate::currency::is_currency_code(to) && to != from {
                    let moved = self.amount_in(budget, &from, to)?;
                    self.set_setting("monthly_budget", &moved.map_or(serde_json::Value::Null, Into::into))?;
                }
            }
        }
        // This device's own: not synced.
        if key == "reminders_on" {
            if !value.is_boolean() {
                return Err(Error::Invalid(format!("bad value for {key}")));
            }
            self.conn.execute(
                "INSERT INTO local_setting (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value.to_string()],
            )?;
            return Ok(());
        }
        let ok = match key {
            "monthly_budget" => value.is_null() || value.as_i64().is_some_and(|n| n > 0),
            "main_currency" | "last_currency" => value.as_str().is_some_and(crate::currency::is_currency_code),
            "reminder_time" => value.as_str().is_some_and(|t| chrono::NaiveTime::parse_from_str(t, "%H:%M").is_ok()),
            "default_remind_days" => value.as_u64().is_some_and(|n| n <= 60),
            _ => return Err(Error::Invalid(format!("unknown setting {key}"))),
        };
        if !ok {
            return Err(Error::Invalid(format!("bad value for {key}")));
        }
        self.set_setting(key, value)
    }

    // ------------------------------------------------------------ backup

    /// Consistent snapshot via `VACUUM INTO`.
    pub fn export_to(&self, path: &str) -> Result<()> {
        // VACUUM INTO refuses to overwrite.
        let _ = std::fs::remove_file(path);
        self.conn.execute("VACUUM INTO ?1", [path])?;
        Ok(())
    }

    /// Replace everything with a snapshot made by [`Store::export_to`]. The
    /// live connection stays open (SQLite backup API), then migrations bring
    /// an older snapshot up to date. Refuses files that aren't Hisaab
    /// databases or come from a newer app version.
    pub fn import_from(&mut self, path: &str) -> Result<()> {
        let src = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| Error::Invalid("not a database file".into()))?;
        let is_hisaab: bool = src
            .query_row("SELECT count(*) = 1 FROM sqlite_master WHERE type = 'table' AND name = 'subscription'", [], |r| {
                r.get(0)
            })
            .map_err(|_| Error::Invalid("not a database file".into()))?;
        if !is_hisaab {
            return Err(Error::Invalid("not a Hisaab backup".into()));
        }
        let version: i64 = src.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let latest: i64 = self.conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > latest {
            return Err(Error::Invalid("backup is from a newer version of Hisaab; update the app first".into()));
        }
        // The snapshot may come from another device: keep this one's sync
        // id, settings and paired devices.
        let sync = self.sync_settings()?;
        let peers = self.peers()?;
        let local: Vec<(String, String)> = self
            .conn
            .prepare("SELECT key, value FROM local_setting")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        // Reminders already shown here stay shown (the snapshot's state is older).
        let fired: Vec<(String, i64)> = self
            .conn
            .prepare("SELECT subscription_id, fired_through FROM reminder_state")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let seq_before = self.last_seq()?.max(self.conn.query_row(
            "SELECT coalesce(max(seq), 0) FROM sqlite_sequence WHERE name = 'changes'",
            [],
            |r| r.get::<_, i64>(0),
        )?);
        rusqlite::backup::Backup::new(&src, &mut self.conn)?
            .run_to_completion(256, std::time::Duration::from_millis(0), None)?;
        schema::migrations().to_latest(&mut self.conn).map_err(|e| Error::Migration(e.to_string()))?;
        self.conn.execute("UPDATE sync_state SET device = ?1, applying = 0", [&sync.device])?;
        self.set_sync_mode(sync.mode)?;
        if !sync.name.is_empty() {
            self.set_device_name(&sync.name)?;
        }
        // Peers already have our changes up to seq_before: new ones must come
        // after it, or they'd never be sent (the snapshot's counter is lower).
        let bumped = self
            .conn
            .execute("UPDATE sqlite_sequence SET seq = max(seq, ?1) WHERE name = 'changes'", [seq_before])?;
        if bumped == 0 {
            self.conn.execute("INSERT INTO sqlite_sequence (name, seq) VALUES ('changes', ?1)", [seq_before])?;
        }
        // Paired devices stay paired, but we ask each for everything again
        // (their_seq 0): what they sent after the snapshot was rolled back
        // here. Merging is per-field newest-wins, so repeats are harmless.
        // Open conflicts in the snapshot are stale (maybe settled since).
        self.conn.execute(
            "UPDATE conflicts SET deleted_at = ?1, updated_at = ?1 WHERE deleted_at IS NULL AND resolution IS NULL",
            [self.now()],
        )?;
        for (sub, at) in fired {
            self.conn.execute(
                "INSERT INTO reminder_state (subscription_id, fired_through) VALUES (?1, ?2)
                 ON CONFLICT(subscription_id) DO UPDATE SET fired_through = max(fired_through, excluded.fired_through)",
                params![sub, at],
            )?;
        }
        self.conn.execute("DELETE FROM local_setting", [])?; // device-local too
        for (k, v) in local {
            self.conn.execute("INSERT INTO local_setting (key, value) VALUES (?1, ?2)", params![k, v])?;
        }
        self.conn.execute("DELETE FROM peers", [])?; // device-local table, replaced wholesale
        for p in peers {
            self.add_peer(&p.endpoint, &p.device, &p.name)?;
            self.set_peer_addrs(&p.endpoint, &p.addrs)?;
            self.set_peer_kind(&p.endpoint, &p.kind)?;
        }
        sync::install(&self.conn)?;
        Ok(())
    }
}
