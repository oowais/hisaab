//! Conflicts (copied from Befikr): this device's edits that lost to a
//! concurrent edit from another device (per-field newest-wins picked theirs).
//! Device-local, not synced: each device lists the edits *it* lost. Review:
//! keep theirs (dismiss), or use mine (written back as a new local edit,
//! which then wins everywhere).
//!
//! "Concurrent" = the incoming change replaced an older version than the
//! one we had (its `prev_hlc` < our version's hlc), so its author never saw
//! our edit. Overwriting an edit you've seen is just editing.
//!
//! Hisaab adds overlaps: both devices changed a subscription's terms
//! offline, so after the merge two periods cover the same days. One entry
//! per subscription (field `overlap`, values = period ids, ours lost);
//! resolving keeps one period and drops the other.

use rusqlite::types::Value as SqlValue;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};

use crate::dates::{next_day, prev_day};
use crate::store::{new_id, Store};
use crate::subscriptions::load_periods;
use crate::sync::{to_json, to_sql, SyncChange, SYNCED};
use crate::{Error, Period, Result};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub id: String,
    pub table: String,
    pub row_id: String,
    /// The field, or `overlap` for two periods covering the same days.
    pub field: String,
    /// What the edit was about: subscription, category or method name, setting key.
    pub label: String,
    /// This device's value that lost. For an overlap: our period
    /// (`{start, end, amountMinor, currency, everyN, unit, status}`).
    pub lost_value: Value,
    pub won_value: Value,
    /// Name of the paired device whose edit won (None: unknown, e.g. it
    /// came via another device).
    pub won_by: Option<String>,
    pub at: i64,
}

/// Fields worth asking about. Timestamps that only say "archived"/"deleted"
/// count when that state differs, not when both are set at different times.
pub(crate) fn worth_logging(table: &str, field: &str, lost: &SqlValue, won: &Value) -> bool {
    let lost = to_json(lost.clone());
    match (table, field) {
        (_, "deleted_at") | ("subscription", "archived_at") => lost.is_null() != won.is_null(),
        (_, "created_at" | "subscription_id" | "period_id" | "sort") => false,
        _ => lost != *won,
    }
}

/// Record that our `lost` value of `ch`'s field lost to `ch`. One open
/// conflict per field: a newer one replaces it.
pub(crate) fn log(c: &Connection, ch: &SyncChange, lost: SqlValue, now: i64) -> Result<()> {
    record(c, &ch.table, &ch.row_id, &ch.field, lost, to_sql(&ch.value), Some(&ch.origin), now)
}

#[allow(clippy::too_many_arguments)]
fn record(
    c: &Connection,
    table: &str,
    row_id: &str,
    field: &str,
    lost: SqlValue,
    won: SqlValue,
    won_origin: Option<&str>,
    now: i64,
) -> Result<()> {
    c.execute(
        "UPDATE conflicts SET deleted_at = ?4, updated_at = ?4
         WHERE tbl = ?1 AND row_id = ?2 AND field = ?3 AND resolution IS NULL AND deleted_at IS NULL",
        params![table, row_id, field, now],
    )?;
    c.execute(
        "INSERT INTO conflicts (id, tbl, row_id, field, lost_value, won_value, won_origin, at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![new_id(), table, row_id, field, lost, won, won_origin, now],
    )?;
    Ok(())
}

/// The device that created a period.
fn period_origin(c: &Connection, id: &str) -> Result<Option<String>> {
    Ok(c
        .query_row(
            "SELECT origin FROM changes WHERE tbl = 'period' AND row_id = ?1 AND field = 'created_at'",
            [id],
            |r| r.get(0),
        )
        .optional()?)
}

/// `a` starts no later than `b`.
fn overlap(a: &Period, b: &Period) -> bool {
    a.end_date.is_none_or(|e| e >= b.start_date)
}

fn overlap_any(a: &Period, b: &Period) -> bool {
    if a.start_date <= b.start_date { overlap(a, b) } else { overlap(b, a) }
}

/// After a merge touched `sub`'s periods: if two of them now overlap, one
/// ours and one from another device, note it (the newest such pair). An
/// open overlap entry that no longer applies is dropped.
pub(crate) fn log_overlaps(c: &Connection, sub: &str, me: &str, now: i64) -> Result<()> {
    let periods = load_periods(c, sub)?; // sorted by start
    let mut found: Option<(&Period, &Period, String)> = None;
    for (i, a) in periods.iter().enumerate() {
        for b in &periods[i + 1..] {
            if !overlap(a, b) {
                continue;
            }
            let (oa, ob) = (period_origin(c, &a.id)?, period_origin(c, &b.id)?);
            let (mine, theirs, origin) = match (oa.as_deref() == Some(me), ob.as_deref() == Some(me)) {
                (true, false) => (a, b, ob),
                (false, true) => (b, a, oa),
                _ => continue,
            };
            // Later pairs start later: the newest pair wins.
            found = Some((mine, theirs, origin.unwrap_or_default()));
        }
    }
    match found {
        Some((mine, theirs, origin)) => {
            // The old period's end moved with it: part of this overlap, not
            // a question of its own.
            c.execute(
                "UPDATE conflicts SET deleted_at = ?2, updated_at = ?2
                 WHERE tbl = 'period' AND field = 'end_date' AND resolution IS NULL AND deleted_at IS NULL
                   AND row_id IN (SELECT id FROM period WHERE subscription_id = ?1)",
                params![sub, now],
            )?;
            record(
                c,
                "subscription",
                sub,
                "overlap",
                mine.id.clone().into(),
                theirs.id.clone().into(),
                Some(&origin).filter(|o| !o.is_empty()).map(|o| o.as_str()),
                now,
            )
        }
        None => {
            c.execute(
                "UPDATE conflicts SET deleted_at = ?2, updated_at = ?2
                 WHERE tbl = 'subscription' AND row_id = ?1 AND field = 'overlap' AND resolution IS NULL AND deleted_at IS NULL",
                params![sub, now],
            )?;
            Ok(())
        }
    }
}

fn period_json(p: &Period) -> Value {
    json!({
        "start": p.start_date,
        "end": p.end_date,
        "amountMinor": p.terms.amount_minor,
        "currency": p.terms.currency,
        "everyN": p.terms.every_n,
        "unit": p.terms.unit,
        "status": p.status,
    })
}

/// Keep `keep`, drop `drop`, and close the period before `keep` the day
/// before it starts when it ran into either of them.
fn settle_overlap(c: &Connection, sub: &str, keep: &str, drop: &str, now: i64) -> Result<()> {
    let periods = load_periods(c, sub)?;
    let (Some(k), Some(d)) = (periods.iter().find(|p| p.id == keep), periods.iter().find(|p| p.id == drop)) else {
        return Ok(()); // one is gone already (settled on the other device)
    };
    c.execute("UPDATE period SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1", params![drop, now])?;
    let before = periods.iter().rev().find(|p| p.id != keep && p.id != drop && p.start_date < k.start_date);
    if let Some(prev) = before {
        let runs_in = prev.end_date.is_none_or(|e| e >= k.start_date || next_day(e) == d.start_date);
        if runs_in {
            c.execute(
                "UPDATE period SET end_date = ?2, updated_at = ?3 WHERE id = ?1",
                params![prev.id, prev_day(k.start_date).to_string(), now],
            )?;
        }
    }
    Ok(())
}

impl Store {
    /// Open conflicts, newest first.
    pub fn conflicts(&self) -> Result<Vec<Conflict>> {
        let mut st = self.conn.prepare(
            "SELECT c.id, c.tbl, c.row_id, c.field, c.lost_value, c.won_value, c.at,
                    (SELECT p.name FROM peers p WHERE p.device = c.won_origin AND p.deleted_at IS NULL),
                    CASE c.tbl
                      WHEN 'subscription' THEN (SELECT s.name FROM subscription s WHERE s.id = c.row_id)
                      WHEN 'period' THEN (SELECT s.name FROM period p JOIN subscription s ON s.id = p.subscription_id WHERE p.id = c.row_id)
                      WHEN 'charge_override' THEN (SELECT s.name FROM charge_override o JOIN period p ON p.id = o.period_id
                                                   JOIN subscription s ON s.id = p.subscription_id WHERE o.id = c.row_id)
                      WHEN 'category' THEN (SELECT name FROM category WHERE id = c.row_id)
                      WHEN 'payment_method' THEN (SELECT name FROM payment_method WHERE id = c.row_id)
                      ELSE c.row_id END
             FROM conflicts c WHERE c.resolution IS NULL AND c.deleted_at IS NULL ORDER BY c.at DESC, c.id DESC",
        )?;
        let rows = st
            .query_map([], |r| {
                Ok(Conflict {
                    id: r.get(0)?,
                    table: r.get(1)?,
                    row_id: r.get(2)?,
                    field: r.get(3)?,
                    lost_value: to_json(r.get(4)?),
                    won_value: to_json(r.get(5)?),
                    at: r.get(6)?,
                    won_by: r.get(7)?,
                    label: r.get::<_, Option<String>>(8)?.unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut out = Vec::with_capacity(rows.len());
        for mut x in rows {
            if x.field == "overlap" {
                // Shown as the two periods; gone once either was dropped.
                let periods = load_periods(&self.conn, &x.row_id)?;
                let find = |v: &Value| periods.iter().find(|p| v.as_str() == Some(&p.id));
                let (Some(mine), Some(theirs)) = (find(&x.lost_value), find(&x.won_value)) else { continue };
                if !overlap_any(mine, theirs) {
                    continue;
                }
                x.lost_value = period_json(mine);
                x.won_value = period_json(theirs);
            }
            out.push(x);
        }
        Ok(out)
    }

    /// Settle a conflict. `use_mine`: write our lost value back (a new local
    /// edit, so it syncs out and wins); otherwise keep theirs. An overlap
    /// keeps our period or theirs and drops the other.
    pub fn resolve_conflict(&mut self, id: &str, use_mine: bool) -> Result<()> {
        let row: Option<(String, String, String, SqlValue, SqlValue)> = self
            .conn
            .query_row(
                "SELECT tbl, row_id, field, lost_value, won_value FROM conflicts
                 WHERE id = ?1 AND resolution IS NULL AND deleted_at IS NULL",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()?;
        let Some((table, row_id, field, lost, won)) = row else { return Err(Error::NotFound(id.to_string())) };
        self.write(|c, now| {
            let resolution = if use_mine { "mine" } else { "theirs" };
            c.execute("UPDATE conflicts SET resolution = ?2, updated_at = ?3 WHERE id = ?1", params![id, resolution, now])?;
            if field == "overlap" {
                let text = |v: SqlValue| match v {
                    SqlValue::Text(s) => Ok(s),
                    _ => Err(Error::Invalid("bad overlap entry".into())),
                };
                let (mine, theirs) = (text(lost)?, text(won)?);
                let (keep, drop) = if use_mine { (mine, theirs) } else { (theirs, mine) };
                return settle_overlap(c, &row_id, &keep, &drop, now);
            }
            if !use_mine {
                return Ok(());
            }
            // Names come from our own table, but only ever touch synced fields.
            let t = SYNCED
                .iter()
                .find(|t| t.table == table && t.fields.contains(&field.as_str()))
                .ok_or_else(|| Error::Invalid(format!("can't restore {table}.{field}")))?;
            c.execute(
                &format!("UPDATE {} SET {field} = ?1, updated_at = ?2 WHERE {} = ?3", t.table, t.key),
                params![lost, now, row_id],
            )?;
            Ok(())
        })
    }
}
