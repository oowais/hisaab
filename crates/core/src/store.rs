//! The repository: owns the SQLite connection; every read and write goes
//! through here.

use rusqlite::{params, Connection, OptionalExtension};

use serde::{Deserialize, Serialize};

use crate::{schema, Error, Result};

/// App-wide settings (synced). Theme and the like are per device (frontend).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Totals and converted amounts are shown in it.
    pub main_currency: String,
    /// Remind this many days before a charge, unless a subscription says otherwise.
    pub default_remind_days: u32,
    /// Quick add's currency when no template says otherwise.
    pub last_currency: Option<String>,
    /// Reminders show at this local time, "HH:MM".
    pub reminder_time: String,
}

pub struct Store {
    pub(crate) conn: Connection,
    clock: Box<dyn Fn() -> i64 + Send>,
}

fn system_now() -> i64 {
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
        })
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
    /// `last_currency`, `reminder_time`).
    pub fn update_setting(&mut self, key: &str, value: &serde_json::Value) -> Result<()> {
        let ok = match key {
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
}
