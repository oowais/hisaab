//! The repository: owns the SQLite connection; every read and write goes
//! through here.

use rusqlite::{params, Connection, OptionalExtension};

use crate::{schema, Error, Result};

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
}
