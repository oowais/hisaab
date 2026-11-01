//! Repositories for categories, payment methods and subscriptions, and the
//! subscription actions. Each action rebuilds the subscription's period list
//! in memory, validates it (invariants.rs) and writes only the difference:
//!
//! | Action | Periods |
//! | --- | --- |
//! | add | 1 open period (active or trial; a trial with an end gets the paid period after it) |
//! | change terms from a date | close the covering period the day before; new period, same anchor |
//! | pause (optional resume date) | close the covering period; paused period (+ active one from the resume date) |
//! | resume | close the paused period; active period anchored on the resume date |
//! | cancel | end the covering period; drop periods after it |
//! | restart | new active period after the gap |
//! | convert trial | close the trial; active period anchored on the conversion date |
//!
//! Overrides follow their charge: after a split, an override moves to the
//! period that now schedules its date, or is dropped when none does.

use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::dates::{next_day, prev_day};
use crate::invariants::{validate_periods, validate_terms};
use crate::model::*;
use crate::occurrence::{is_scheduled, next_charge, occurrences, with_extras};
use crate::store::{new_id, Store};
use crate::{Error, Result};

/// A new subscription and its first stretch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSubscription {
    pub fields: SubscriptionFields,
    pub terms: Terms,
    pub start_date: NaiveDate,
    /// First charge; default: the start date.
    pub anchor_date: Option<NaiveDate>,
    /// Starts with a free trial ending on this day; the paid period with
    /// `terms` follows from the next day.
    pub trial_end: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewOverride {
    pub period_id: String,
    pub due_date: NaiveDate,
    pub kind: OverrideKind,
    pub new_due_date: Option<NaiveDate>,
    pub actual_amount_minor: Option<i64>,
    pub actual_currency: Option<String>,
    pub settled_amount_minor: Option<i64>,
    pub settled_currency: Option<String>,
}

/// A one-time charge to add (`ExtraCharge` without its id).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewExtraCharge {
    pub subscription_id: String,
    pub due_date: NaiveDate,
    pub amount_minor: i64,
    pub currency: String,
    #[serde(default)]
    pub label: String,
}

fn date(s: &str) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))
}

fn opt_date(s: Option<String>) -> rusqlite::Result<Option<NaiveDate>> {
    s.as_deref().map(date).transpose()
}

fn text_err(e: Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
}

const SUB_COLS: &str = "id, kind, name, template_key, icon, color, category_id, payment_method_id, url, notes, \
    remind_days_before, archived_at, my_share";

fn sub_row(r: &Row) -> rusqlite::Result<Subscription> {
    Ok(Subscription {
        id: r.get(0)?,
        kind: Kind::parse(&r.get::<_, String>(1)?).map_err(text_err)?,
        name: r.get(2)?,
        template_key: r.get(3)?,
        icon: r.get(4)?,
        color: r.get(5)?,
        category_id: r.get(6)?,
        payment_method_id: r.get(7)?,
        url: r.get(8)?,
        notes: r.get(9)?,
        remind_days_before: r.get(10)?,
        archived_at: r.get(11)?,
        my_share: r.get(12)?,
    })
}

const PERIOD_COLS: &str = "id, subscription_id, status, plan_label, amount_minor, currency, every_n, unit, \
    anchor_date, start_date, end_date, amount_kind";

fn period_row(r: &Row) -> rusqlite::Result<Period> {
    Ok(Period {
        id: r.get(0)?,
        subscription_id: r.get(1)?,
        status: PeriodStatus::parse(&r.get::<_, String>(2)?).map_err(text_err)?,
        terms: Terms {
            plan_label: r.get(3)?,
            amount_minor: r.get(4)?,
            currency: r.get(5)?,
            every_n: r.get(6)?,
            unit: Unit::parse(&r.get::<_, String>(7)?).map_err(text_err)?,
            amount_kind: AmountKind::parse(&r.get::<_, String>(11)?).map_err(text_err)?,
        },
        anchor_date: date(&r.get::<_, String>(8)?)?,
        start_date: date(&r.get::<_, String>(9)?)?,
        end_date: opt_date(r.get(10)?)?,
    })
}

const OVERRIDE_COLS: &str = "o.id, o.period_id, o.due_date, o.kind, o.new_due_date, o.actual_amount_minor, \
    o.actual_currency, o.settled_amount_minor, o.settled_currency";

fn override_row(r: &Row) -> rusqlite::Result<ChargeOverride> {
    Ok(ChargeOverride {
        id: r.get(0)?,
        period_id: r.get(1)?,
        due_date: date(&r.get::<_, String>(2)?)?,
        kind: OverrideKind::parse(&r.get::<_, String>(3)?).map_err(text_err)?,
        new_due_date: opt_date(r.get(4)?)?,
        actual_amount_minor: r.get(5)?,
        actual_currency: r.get(6)?,
        settled_amount_minor: r.get(7)?,
        settled_currency: r.get(8)?,
    })
}

pub(crate) fn load_periods(c: &Connection, sub_id: &str) -> Result<Vec<Period>> {
    let mut st = c.prepare(&format!(
        "SELECT {PERIOD_COLS} FROM period WHERE subscription_id = ?1 AND deleted_at IS NULL ORDER BY start_date"
    ))?;
    let rows = st.query_map([sub_id], period_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn load_overrides(c: &Connection, sub_id: &str) -> Result<Vec<ChargeOverride>> {
    let mut st = c.prepare(&format!(
        "SELECT {OVERRIDE_COLS} FROM charge_override o JOIN period p ON p.id = o.period_id
         WHERE p.subscription_id = ?1 AND o.deleted_at IS NULL AND p.deleted_at IS NULL ORDER BY o.due_date"
    ))?;
    let rows = st.query_map([sub_id], override_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

const EXTRA_COLS: &str = "id, subscription_id, due_date, amount_minor, currency, label";

fn extra_row(r: &Row) -> rusqlite::Result<ExtraCharge> {
    Ok(ExtraCharge {
        id: r.get(0)?,
        subscription_id: r.get(1)?,
        due_date: date(&r.get::<_, String>(2)?)?,
        amount_minor: r.get(3)?,
        currency: r.get(4)?,
        label: r.get(5)?,
    })
}

fn load_extras(c: &Connection, sub_id: &str) -> Result<Vec<ExtraCharge>> {
    let mut st = c.prepare(&format!(
        "SELECT {EXTRA_COLS} FROM extra_charge WHERE subscription_id = ?1 AND deleted_at IS NULL ORDER BY due_date, id"
    ))?;
    let rows = st.query_map([sub_id], extra_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn check_extra(amount_minor: i64, currency: &str) -> Result<()> {
    if amount_minor <= 0 {
        return Err(Error::Invalid("amount must be more than 0".into()));
    }
    if !crate::currency::is_currency_code(currency) {
        return Err(Error::Invalid(format!("unknown currency {currency:?}")));
    }
    Ok(())
}

fn load_subscription(c: &Connection, id: &str) -> Result<Subscription> {
    c.query_row(&format!("SELECT {SUB_COLS} FROM subscription WHERE id = ?1 AND deleted_at IS NULL"), [id], sub_row)
        .optional()?
        .ok_or_else(|| Error::NotFound(id.into()))
}

fn insert_period(c: &Connection, p: &Period, now: i64) -> Result<()> {
    c.execute(
        &format!("INSERT INTO period ({PERIOD_COLS}, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)"),
        params![
            p.id,
            p.subscription_id,
            p.status.as_str(),
            p.terms.plan_label,
            p.terms.amount_minor,
            p.terms.currency,
            p.terms.every_n,
            p.terms.unit.as_str(),
            p.anchor_date.to_string(),
            p.start_date.to_string(),
            p.end_date.map(|d| d.to_string()),
            p.terms.amount_kind.as_str(),
            now,
        ],
    )?;
    Ok(())
}

fn update_period(c: &Connection, p: &Period, now: i64) -> Result<()> {
    c.execute(
        "UPDATE period SET status = ?2, plan_label = ?3, amount_minor = ?4, currency = ?5, every_n = ?6, unit = ?7,
           anchor_date = ?8, start_date = ?9, end_date = ?10, amount_kind = ?11, updated_at = ?12 WHERE id = ?1",
        params![
            p.id,
            p.status.as_str(),
            p.terms.plan_label,
            p.terms.amount_minor,
            p.terms.currency,
            p.terms.every_n,
            p.terms.unit.as_str(),
            p.anchor_date.to_string(),
            p.start_date.to_string(),
            p.end_date.map(|d| d.to_string()),
            p.terms.amount_kind.as_str(),
            now,
        ],
    )?;
    Ok(())
}

/// Validate `after` and write what changed from `before`; re-home or drop
/// the subscription's overrides to match.
fn save_periods(c: &Connection, before: &[Period], after: &[Period], now: i64) -> Result<()> {
    validate_periods(after)?;
    for p in after {
        match before.iter().find(|b| b.id == p.id) {
            None => insert_period(c, p, now)?,
            Some(b) if b != p => update_period(c, p, now)?,
            Some(_) => {}
        }
    }
    for b in before.iter().filter(|b| !after.iter().any(|p| p.id == b.id)) {
        c.execute("UPDATE period SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1", params![b.id, now])?;
    }
    let Some(sub_id) = after.first().or(before.first()).map(|p| p.subscription_id.clone()) else {
        return Ok(());
    };
    // Overrides of periods just deleted are gone from load_overrides; read
    // them directly by the old period ids too.
    let ids: Vec<&str> = before.iter().map(|p| p.id.as_str()).collect();
    let mut overrides = load_overrides(c, &sub_id)?;
    for id in ids.iter().filter(|id| !after.iter().any(|p| p.id == **id)) {
        let mut st = c.prepare(&format!(
            "SELECT {OVERRIDE_COLS} FROM charge_override o WHERE o.period_id = ?1 AND o.deleted_at IS NULL"
        ))?;
        overrides.extend(st.query_map([id], override_row)?.collect::<rusqlite::Result<Vec<_>>>()?);
    }
    for o in overrides {
        let home = after.iter().find(|p| p.covers(o.due_date) && is_scheduled(p, o.due_date));
        match home {
            Some(p) if p.id == o.period_id => {}
            Some(p) => {
                c.execute(
                    "UPDATE charge_override SET period_id = ?2, updated_at = ?3 WHERE id = ?1",
                    params![o.id, p.id, now],
                )?;
            }
            None => {
                c.execute("UPDATE charge_override SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1", params![o.id, now])?;
            }
        }
    }
    Ok(())
}

fn validate_fields(f: &SubscriptionFields) -> Result<()> {
    if f.name.trim().is_empty() {
        return Err(Error::Invalid("name is empty".into()));
    }
    if f.my_share.is_some_and(|p| !(1..=99).contains(&p)) {
        return Err(Error::Invalid("your share must be 1–99%".into()));
    }
    Ok(())
}

/// Index of the period covering `day`.
fn covering(periods: &[Period], day: NaiveDate) -> Result<usize> {
    periods
        .iter()
        .position(|p| p.covers(day))
        .ok_or_else(|| Error::Invalid(format!("not subscribed on {day}")))
}

/// Split `periods[i]` at `at`: it keeps the days before `at` (same id) and a
/// copy with a new id starts on `at`. Returns the index of the part starting
/// on `at`. Splitting at the period's start splits nothing.
fn split(periods: &mut Vec<Period>, i: usize, at: NaiveDate) -> usize {
    if periods[i].start_date == at {
        return i;
    }
    let mut right = periods[i].clone();
    right.id = new_id();
    right.start_date = at;
    periods[i].end_date = Some(prev_day(at));
    periods.insert(i + 1, right);
    i + 1
}

/// Live subscriptions whose `col` is `id`.
fn ids_where(c: &Connection, col: &str, id: &str) -> Result<Vec<String>> {
    let mut st = c.prepare(&format!("SELECT id FROM subscription WHERE {col} = ?1 AND deleted_at IS NULL"))?;
    let rows = st.query_map([id], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

impl Store {
    // ---- Categories and payment methods ----

    pub fn add_category(&mut self, name: &str, color: &str) -> Result<Category> {
        if name.trim().is_empty() {
            return Err(Error::Invalid("name is empty".into()));
        }
        let id = new_id();
        self.write(|c, now| {
            let sort: i64 = c.query_row("SELECT COALESCE(MAX(sort), -1) + 1 FROM category", [], |r| r.get(0))?;
            c.execute(
                "INSERT INTO category (id, name, color, sort, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![id, name.trim(), color, sort, now],
            )?;
            Ok(Category { id: id.clone(), name: name.trim().into(), color: color.into(), sort })
        })
    }

    /// A pre-made category, picked for the first time (or again after it was
    /// deleted). Its id is fixed in the app, so two devices that pick it end
    /// up with one category after syncing. Kept as is when it exists.
    pub fn ensure_category(&mut self, id: &str, name: &str, color: &str) -> Result<Category> {
        if id.trim().is_empty() || name.trim().is_empty() {
            return Err(Error::Invalid("id or name is empty".into()));
        }
        self.write(|c, now| {
            let found: Option<(Option<i64>, String, String, i64)> = c
                .query_row("SELECT deleted_at, name, color, sort FROM category WHERE id = ?1", [id], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                })
                .optional()?;
            let sort = match found {
                Some((None, name, color, sort)) => return Ok(Category { id: id.into(), name, color, sort }),
                Some((Some(_), name, color, sort)) => {
                    c.execute("UPDATE category SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1", params![id, now])?;
                    return Ok(Category { id: id.into(), name, color, sort });
                }
                None => c.query_row("SELECT COALESCE(MAX(sort), -1) + 1 FROM category", [], |r| r.get(0))?,
            };
            c.execute(
                "INSERT INTO category (id, name, color, sort, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![id, name.trim(), color, sort, now],
            )?;
            Ok(Category { id: id.into(), name: name.trim().into(), color: color.into(), sort })
        })
    }

    pub fn categories(&self) -> Result<Vec<Category>> {
        let mut st = self
            .conn
            .prepare("SELECT id, name, color, sort FROM category WHERE deleted_at IS NULL ORDER BY sort, name")?;
        let rows = st.query_map([], |r| Ok(Category { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, sort: r.get(3)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn update_category(&mut self, cat: &Category) -> Result<()> {
        if cat.name.trim().is_empty() {
            return Err(Error::Invalid("name is empty".into()));
        }
        self.write(|c, now| {
            let n = c.execute(
                "UPDATE category SET name = ?2, color = ?3, sort = ?4, updated_at = ?5 WHERE id = ?1 AND deleted_at IS NULL",
                params![cat.id, cat.name.trim(), cat.color, cat.sort, now],
            )?;
            if n == 0 {
                return Err(Error::NotFound(cat.id.clone()));
            }
            Ok(())
        })
    }

    /// Soft delete; subscriptions in it become uncategorised. Returns them,
    /// for [`Store::restore_category`] (undo).
    pub fn delete_category(&mut self, id: &str) -> Result<Vec<String>> {
        self.write(|c, now| {
            let subs = ids_where(c, "category_id", id)?;
            c.execute("UPDATE category SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1", params![id, now])?;
            c.execute(
                "UPDATE subscription SET category_id = NULL, updated_at = ?2 WHERE category_id = ?1",
                params![id, now],
            )?;
            Ok(subs)
        })
    }

    /// Undo [`Store::delete_category`]: back, with `subs` in it again.
    pub fn restore_category(&mut self, id: &str, subs: &[String]) -> Result<()> {
        self.write(|c, now| {
            c.execute("UPDATE category SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1", params![id, now])?;
            for s in subs {
                c.execute(
                    "UPDATE subscription SET category_id = ?2, updated_at = ?3 WHERE id = ?1 AND category_id IS NULL",
                    params![s, id, now],
                )?;
            }
            Ok(())
        })
    }

    pub fn add_payment_method(&mut self, name: &str) -> Result<PaymentMethod> {
        if name.trim().is_empty() {
            return Err(Error::Invalid("name is empty".into()));
        }
        let id = new_id();
        self.write(|c, now| {
            c.execute(
                "INSERT INTO payment_method (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
                params![id, name.trim(), now],
            )?;
            Ok(PaymentMethod { id: id.clone(), name: name.trim().into() })
        })
    }

    pub fn payment_methods(&self) -> Result<Vec<PaymentMethod>> {
        let mut st =
            self.conn.prepare("SELECT id, name FROM payment_method WHERE deleted_at IS NULL ORDER BY name")?;
        let rows = st.query_map([], |r| Ok(PaymentMethod { id: r.get(0)?, name: r.get(1)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn rename_payment_method(&mut self, id: &str, name: &str) -> Result<()> {
        if name.trim().is_empty() {
            return Err(Error::Invalid("name is empty".into()));
        }
        self.write(|c, now| {
            let n = c.execute(
                "UPDATE payment_method SET name = ?2, updated_at = ?3 WHERE id = ?1 AND deleted_at IS NULL",
                params![id, name.trim(), now],
            )?;
            if n == 0 {
                return Err(Error::NotFound(id.into()));
            }
            Ok(())
        })
    }

    /// Soft delete; subscriptions paid with it lose the link. Returns them,
    /// for [`Store::restore_payment_method`] (undo).
    pub fn delete_payment_method(&mut self, id: &str) -> Result<Vec<String>> {
        self.write(|c, now| {
            let subs = ids_where(c, "payment_method_id", id)?;
            c.execute("UPDATE payment_method SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1", params![id, now])?;
            c.execute(
                "UPDATE subscription SET payment_method_id = NULL, updated_at = ?2 WHERE payment_method_id = ?1",
                params![id, now],
            )?;
            Ok(subs)
        })
    }

    /// Undo [`Store::delete_payment_method`]: back, linked to `subs` again.
    pub fn restore_payment_method(&mut self, id: &str, subs: &[String]) -> Result<()> {
        self.write(|c, now| {
            c.execute("UPDATE payment_method SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1", params![id, now])?;
            for s in subs {
                c.execute(
                    "UPDATE subscription SET payment_method_id = ?2, updated_at = ?3 WHERE id = ?1 AND payment_method_id IS NULL",
                    params![s, id, now],
                )?;
            }
            Ok(())
        })
    }

    /// Reorder categories: `ids` in their new order.
    pub fn sort_categories(&mut self, ids: &[String]) -> Result<()> {
        self.write(|c, now| {
            for (i, id) in ids.iter().enumerate() {
                c.execute(
                    "UPDATE category SET sort = ?2, updated_at = ?3 WHERE id = ?1 AND sort IS NOT ?2",
                    params![id, i as i64, now],
                )?;
            }
            Ok(())
        })
    }

    // ---- Subscriptions ----

    pub fn add_subscription(&mut self, new: &NewSubscription) -> Result<SubscriptionDetail> {
        let f = &new.fields;
        validate_fields(f)?;
        validate_terms(&new.terms)?;
        let id = new_id();
        let mut periods = Vec::new();
        let paid_start = match new.trial_end {
            Some(end) => {
                if end < new.start_date {
                    return Err(Error::Invalid("trial ends before it starts".into()));
                }
                periods.push(Period {
                    id: new_id(),
                    subscription_id: id.clone(),
                    status: PeriodStatus::Trial,
                    terms: Terms { amount_minor: 0, ..new.terms.clone() },
                    anchor_date: new.start_date,
                    start_date: new.start_date,
                    end_date: Some(end),
                });
                next_day(end)
            }
            None => new.start_date,
        };
        periods.push(Period {
            id: new_id(),
            subscription_id: id.clone(),
            status: PeriodStatus::Active,
            terms: new.terms.clone(),
            anchor_date: new.anchor_date.filter(|_| new.trial_end.is_none()).unwrap_or(paid_start),
            start_date: paid_start,
            end_date: None,
        });
        self.write(|c, now| {
            c.execute(
                &format!("INSERT INTO subscription ({SUB_COLS}, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, ?13, ?12, ?12)"),
                params![
                    id,
                    f.kind.unwrap_or(Kind::Subscription).as_str(),
                    f.name.trim(),
                    f.template_key,
                    f.icon,
                    f.color,
                    f.category_id,
                    f.payment_method_id,
                    f.url,
                    f.notes,
                    f.remind_days_before,
                    now,
                    f.my_share,
                ],
            )?;
            save_periods(c, &[], &periods, now)
        })?;
        self.subscription(&id)
    }

    pub fn update_subscription(&mut self, id: &str, f: &SubscriptionFields) -> Result<()> {
        validate_fields(f)?;
        self.write(|c, now| {
            let n = c.execute(
                "UPDATE subscription SET kind = ?2, name = ?3, template_key = ?4, icon = ?5, color = ?6, category_id = ?7,
                   payment_method_id = ?8, url = ?9, notes = ?10, remind_days_before = ?11, updated_at = ?12,
                   my_share = ?13
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![
                    id,
                    f.kind.unwrap_or(Kind::Subscription).as_str(),
                    f.name.trim(),
                    f.template_key,
                    f.icon,
                    f.color,
                    f.category_id,
                    f.payment_method_id,
                    f.url,
                    f.notes,
                    f.remind_days_before,
                    now,
                    f.my_share,
                ],
            )?;
            if n == 0 {
                return Err(Error::NotFound(id.into()));
            }
            Ok(())
        })
    }

    pub fn subscription(&self, id: &str) -> Result<SubscriptionDetail> {
        Ok(SubscriptionDetail {
            subscription: load_subscription(&self.conn, id)?,
            periods: load_periods(&self.conn, id)?,
            overrides: load_overrides(&self.conn, id)?,
            extras: load_extras(&self.conn, id)?,
        })
    }

    /// All subscriptions (archived too), by name.
    pub fn subscriptions(&self) -> Result<Vec<SubscriptionDetail>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {SUB_COLS} FROM subscription WHERE deleted_at IS NULL ORDER BY name COLLATE NOCASE"
        ))?;
        let subs = st.query_map([], sub_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        subs.into_iter()
            .map(|s| {
                Ok(SubscriptionDetail {
                    periods: load_periods(&self.conn, &s.id)?,
                    overrides: load_overrides(&self.conn, &s.id)?,
                    extras: load_extras(&self.conn, &s.id)?,
                    subscription: s,
                })
            })
            .collect()
    }

    pub fn set_archived(&mut self, id: &str, archived: bool) -> Result<()> {
        self.write(|c, now| {
            let n = c.execute(
                "UPDATE subscription SET archived_at = ?2, updated_at = ?3 WHERE id = ?1 AND deleted_at IS NULL",
                params![id, archived.then_some(now), now],
            )?;
            if n == 0 {
                return Err(Error::NotFound(id.into()));
            }
            Ok(())
        })
    }

    /// Soft delete with its periods and overrides (undo: `restore_subscription`).
    pub fn delete_subscription(&mut self, id: &str) -> Result<()> {
        self.set_deleted(id, true)
    }

    pub fn restore_subscription(&mut self, id: &str) -> Result<()> {
        self.set_deleted(id, false)
    }

    fn set_deleted(&mut self, id: &str, deleted: bool) -> Result<()> {
        self.write(|c, now| {
            // Restore only what the delete took: rows deleted at the same time.
            let at: Option<i64> = if deleted {
                None
            } else {
                c.query_row("SELECT deleted_at FROM subscription WHERE id = ?1", [id], |r| r.get(0))
                    .optional()?
                    .flatten()
            };
            let (set, cond) = if deleted { (Some(now), "deleted_at IS NULL") } else { (None, "deleted_at = ?3") };
            let n = c.execute(
                &format!("UPDATE subscription SET deleted_at = ?2, updated_at = ?4 WHERE id = ?1 AND {cond}"),
                params![id, set, at, now],
            )?;
            if n == 0 {
                return Err(Error::NotFound(id.into()));
            }
            c.execute(
                &format!(
                    "UPDATE charge_override SET deleted_at = ?2, updated_at = ?4 WHERE {cond}
                     AND period_id IN (SELECT id FROM period WHERE subscription_id = ?1)"
                ),
                params![id, set, at, now],
            )?;
            c.execute(
                &format!("UPDATE period SET deleted_at = ?2, updated_at = ?4 WHERE subscription_id = ?1 AND {cond}"),
                params![id, set, at, now],
            )?;
            Ok(())
        })
    }

    /// Apply `f` to the subscription's periods and save the result.
    fn edit_periods(&mut self, id: &str, f: impl FnOnce(&mut Vec<Period>) -> Result<()>) -> Result<SubscriptionDetail> {
        self.write(|c, now| {
            load_subscription(c, id)?;
            let before = load_periods(c, id)?;
            let mut after = before.clone();
            f(&mut after)?;
            save_periods(c, &before, &after, now)
        })?;
        self.subscription(id)
    }

    /// New price / plan / interval from `from` on (default in the UI: the
    /// next charge). Keeps the anchor day; later scheduled periods stay.
    pub fn change_terms(&mut self, id: &str, from: NaiveDate, terms: &Terms) -> Result<SubscriptionDetail> {
        validate_terms(terms)?;
        self.edit_periods(id, |ps| {
            let i = covering(ps, from)?;
            if ps[i].status == PeriodStatus::Paused {
                return Err(Error::Invalid("can't change the price while paused".into()));
            }
            let j = split(ps, i, from);
            ps[j].terms = terms.clone();
            Ok(())
        })
    }

    /// Fix when a stretch started or ended (the real start, the real
    /// cancellation). `start` only for a period that doesn't follow on from
    /// the one before (the first, or one after a break); `end` only for one
    /// the next doesn't follow on from (cancelled, or before a break); None
    /// = still running (the latest only). A billing day that began with the
    /// period moves with its start.
    pub fn set_period_dates(
        &mut self,
        id: &str,
        period_id: &str,
        start: NaiveDate,
        end: Option<NaiveDate>,
    ) -> Result<SubscriptionDetail> {
        self.edit_periods(id, |ps| {
            let i = ps.iter().position(|p| p.id == period_id).ok_or_else(|| Error::NotFound(period_id.into()))?;
            let follows = |a: &Period, b: &Period| a.end_date.and_then(|e| e.succ_opt()) == Some(b.start_date);
            if start != ps[i].start_date {
                if i > 0 && follows(&ps[i - 1], &ps[i]) {
                    return Err(Error::Invalid("it starts where the period before ends".into()));
                }
                if ps[i].anchor_date == ps[i].start_date {
                    ps[i].anchor_date = start;
                }
                ps[i].start_date = start;
            }
            if end != ps[i].end_date {
                if ps.get(i + 1).is_some_and(|n| follows(&ps[i], n)) {
                    return Err(Error::Invalid("it ends where the next period starts".into()));
                }
                ps[i].end_date = end;
            }
            Ok(())
        })
    }

    /// Charges from `next` on fall on its day (every `every_n unit` from it);
    /// earlier ones stay as they were. Later scheduled price changes that
    /// followed the old day follow the new one. `next`: today or later.
    pub fn change_billing_day(&mut self, id: &str, next: NaiveDate, today: NaiveDate) -> Result<SubscriptionDetail> {
        if next < today {
            return Err(Error::Invalid("pick today or a later day".into()));
        }
        // From tomorrow (today's charge, if any, already happened), or from
        // the new day itself when that's today.
        let from = next.min(next_day(today));
        self.edit_periods(id, |ps| {
            let i = covering(ps, from)?;
            match ps[i].status {
                PeriodStatus::Paused => return Err(Error::Invalid("resume it first".into())),
                PeriodStatus::Trial if !ps[i].charges() => {
                    return Err(Error::Invalid("change it once the free trial is over".into()))
                }
                _ => {}
            }
            let old = ps[i].anchor_date;
            let j = split(ps, i, from);
            ps[j].anchor_date = next;
            for p in ps.iter_mut().skip(j + 1) {
                if p.status != PeriodStatus::Active || p.anchor_date != old {
                    break;
                }
                p.anchor_date = next;
            }
            Ok(())
        })
    }

    /// Pause from `from`; with `resume_on`, charges start again that day
    /// (anchored on it). Without one, the pause is open-ended and later
    /// scheduled changes are dropped.
    pub fn pause(&mut self, id: &str, from: NaiveDate, resume_on: Option<NaiveDate>) -> Result<SubscriptionDetail> {
        if resume_on.is_some_and(|r| r <= from) {
            return Err(Error::Invalid("resume date must be after the pause starts".into()));
        }
        self.edit_periods(id, |ps| {
            let i = covering(ps, from)?;
            if ps[i].status == PeriodStatus::Paused {
                return Err(Error::Invalid("already paused".into()));
            }
            let old_end = ps[i].end_date;
            let j = split(ps, i, from);
            ps[j].status = PeriodStatus::Paused;
            match resume_on {
                Some(r) if old_end.is_none_or(|e| r <= e) => {
                    let mut resumed = ps[j].clone();
                    ps[j].end_date = Some(prev_day(r));
                    resumed.id = new_id();
                    resumed.status = PeriodStatus::Active;
                    resumed.start_date = r;
                    resumed.anchor_date = r;
                    resumed.end_date = old_end;
                    ps.insert(j + 1, resumed);
                }
                Some(_) => {}
                None => {
                    ps[j].end_date = None;
                    ps.truncate(j + 1);
                }
            }
            Ok(())
        })
    }

    /// End a pause: charges start again on `on`, anchored on `anchor` (default `on`).
    pub fn resume(&mut self, id: &str, on: NaiveDate, anchor: Option<NaiveDate>) -> Result<SubscriptionDetail> {
        self.edit_periods(id, |ps| {
            let i = covering(ps, on)?;
            if ps[i].status != PeriodStatus::Paused {
                return Err(Error::Invalid("not paused".into()));
            }
            let j = split(ps, i, on);
            ps[j].status = PeriodStatus::Active;
            ps[j].anchor_date = anchor.unwrap_or(on);
            Ok(())
        })
    }

    /// Ends on `ends_on` (default in the UI: the end of the current billing
    /// cycle). Periods after it are dropped.
    pub fn cancel(&mut self, id: &str, ends_on: NaiveDate) -> Result<SubscriptionDetail> {
        self.edit_periods(id, |ps| {
            let i = covering(ps, ends_on)?;
            ps[i].end_date = Some(ends_on);
            ps.truncate(i + 1);
            Ok(())
        })
    }

    /// Start again on `on` after it ended, with `terms` (default: the last
    /// paid terms), anchored on `on`.
    pub fn restart(&mut self, id: &str, on: NaiveDate, terms: Option<&Terms>) -> Result<SubscriptionDetail> {
        if let Some(t) = terms {
            validate_terms(t)?;
        }
        self.edit_periods(id, |ps| {
            if ps.iter().any(|p| p.end_date.is_none_or(|e| e >= on)) {
                return Err(Error::Invalid("still subscribed then".into()));
            }
            let last = ps
                .iter()
                .rev()
                .find(|p| p.status != PeriodStatus::Trial)
                .or(ps.last())
                .ok_or_else(|| Error::Invalid("nothing to restart".into()))?;
            let period = Period {
                id: new_id(),
                subscription_id: last.subscription_id.clone(),
                status: PeriodStatus::Active,
                terms: terms.cloned().unwrap_or_else(|| last.terms.clone()),
                anchor_date: on,
                start_date: on,
                end_date: None,
            };
            ps.push(period);
            Ok(())
        })
    }

    /// Trial → paid from `on` (first charge that day), with `terms` if they
    /// differ. A trial already followed by a paid period just ends earlier.
    pub fn convert_trial(&mut self, id: &str, on: NaiveDate, terms: Option<&Terms>) -> Result<SubscriptionDetail> {
        if let Some(t) = terms {
            validate_terms(t)?;
        }
        self.edit_periods(id, |ps| {
            let i = covering(ps, on)?;
            if ps[i].status != PeriodStatus::Trial {
                return Err(Error::Invalid("not in a trial".into()));
            }
            // The paid period that follows the trial, if any, supplies the terms.
            let next = ps.get(i + 1).filter(|n| ps[i].end_date.is_some_and(|e| next_day(e) == n.start_date)).cloned();
            let j = split(ps, i, on);
            ps[j].status = PeriodStatus::Active;
            ps[j].anchor_date = on;
            if let Some(t) = terms.or(next.as_ref().map(|n| &n.terms)) {
                ps[j].terms = t.clone();
            }
            if let Some(n) = next {
                // Merge: the paid stretch now starts at `on`.
                ps[j].end_date = n.end_date;
                ps.retain(|p| p.id != n.id);
            }
            Ok(())
        })
    }

    // ---- Overrides ----

    /// Record how one charge differed (replaces an earlier override of it).
    pub fn set_override(&mut self, o: &NewOverride) -> Result<ChargeOverride> {
        match o.kind {
            OverrideKind::Moved if o.new_due_date.is_none() => {
                return Err(Error::Invalid("a moved charge needs its new date".into()))
            }
            OverrideKind::AmountChanged if o.actual_amount_minor.is_none() => {
                return Err(Error::Invalid("a changed charge needs its amount".into()))
            }
            OverrideKind::Settled if o.settled_amount_minor.is_none() => {
                return Err(Error::Invalid("a settled charge needs the amount taken".into()))
            }
            _ => {}
        }
        for (amount, cur) in [(o.actual_amount_minor, &o.actual_currency), (o.settled_amount_minor, &o.settled_currency)] {
            if amount.is_some_and(|a| a < 0) {
                return Err(Error::Invalid("amount can't be negative".into()));
            }
            if cur.as_deref().is_some_and(|c| !crate::currency::is_currency_code(c)) {
                return Err(Error::Invalid(format!("unknown currency {cur:?}")));
            }
        }
        let id = self.write(|c, now| {
            let p = c
                .query_row(
                    &format!("SELECT {PERIOD_COLS} FROM period WHERE id = ?1 AND deleted_at IS NULL"),
                    [&o.period_id],
                    period_row,
                )
                .optional()?
                .ok_or_else(|| Error::NotFound(o.period_id.clone()))?;
            if !is_scheduled(&p, o.due_date) {
                return Err(Error::Invalid(format!("no charge on {}", o.due_date)));
            }
            let existing: Option<String> = c
                .query_row(
                    "SELECT id FROM charge_override WHERE period_id = ?1 AND due_date = ?2 AND deleted_at IS NULL",
                    params![o.period_id, o.due_date.to_string()],
                    |r| r.get(0),
                )
                .optional()?;
            let id = existing.unwrap_or_else(new_id);
            c.execute(
                "INSERT INTO charge_override (id, period_id, due_date, kind, new_due_date, actual_amount_minor,
                   actual_currency, settled_amount_minor, settled_currency, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
                 ON CONFLICT(id) DO UPDATE SET kind = excluded.kind, new_due_date = excluded.new_due_date,
                   actual_amount_minor = excluded.actual_amount_minor, actual_currency = excluded.actual_currency,
                   settled_amount_minor = excluded.settled_amount_minor, settled_currency = excluded.settled_currency,
                   updated_at = excluded.updated_at",
                params![
                    id,
                    o.period_id,
                    o.due_date.to_string(),
                    o.kind.as_str(),
                    o.new_due_date.map(|d| d.to_string()),
                    o.actual_amount_minor,
                    o.actual_currency,
                    o.settled_amount_minor,
                    o.settled_currency,
                    now,
                ],
            )?;
            Ok(id)
        })?;
        Ok(self.conn.query_row(
            &format!("SELECT {OVERRIDE_COLS} FROM charge_override o WHERE o.id = ?1"),
            [&id],
            override_row,
        )?)
    }

    /// Back to what the rule says for that charge.
    pub fn delete_override(&mut self, id: &str) -> Result<()> {
        self.write(|c, now| {
            let n = c.execute(
                "UPDATE charge_override SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
                params![id, now],
            )?;
            if n == 0 {
                return Err(Error::NotFound(id.into()));
            }
            Ok(())
        })
    }

    // ---- One-time charges ----

    pub fn add_extra(&mut self, e: &NewExtraCharge) -> Result<ExtraCharge> {
        check_extra(e.amount_minor, &e.currency)?;
        load_subscription(&self.conn, &e.subscription_id)?;
        let id = new_id();
        self.write(|c, now| {
            c.execute(
                "INSERT INTO extra_charge (id, subscription_id, due_date, amount_minor, currency, label, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![id, e.subscription_id, e.due_date.to_string(), e.amount_minor, e.currency, e.label.trim(), now],
            )?;
            Ok(())
        })?;
        Ok(ExtraCharge {
            id,
            subscription_id: e.subscription_id.clone(),
            due_date: e.due_date,
            amount_minor: e.amount_minor,
            currency: e.currency.clone(),
            label: e.label.trim().into(),
        })
    }

    /// Change a one-time charge's date, amount, currency or note.
    pub fn update_extra(&mut self, e: &ExtraCharge) -> Result<()> {
        check_extra(e.amount_minor, &e.currency)?;
        self.write(|c, now| {
            let n = c.execute(
                "UPDATE extra_charge SET due_date = ?2, amount_minor = ?3, currency = ?4, label = ?5, updated_at = ?6
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![e.id, e.due_date.to_string(), e.amount_minor, e.currency, e.label.trim(), now],
            )?;
            if n == 0 {
                return Err(Error::NotFound(e.id.clone()));
            }
            Ok(())
        })
    }

    /// Soft delete; [`Store::restore_extra`] undoes it.
    pub fn delete_extra(&mut self, id: &str) -> Result<()> {
        self.set_extra_deleted(id, true)
    }

    pub fn restore_extra(&mut self, id: &str) -> Result<()> {
        self.set_extra_deleted(id, false)
    }

    fn set_extra_deleted(&mut self, id: &str, deleted: bool) -> Result<()> {
        self.write(|c, now| {
            let n = c.execute(
                "UPDATE extra_charge SET deleted_at = ?2, updated_at = ?3 WHERE id = ?1 AND (deleted_at IS NULL) = ?4",
                params![id, deleted.then_some(now), now, deleted],
            )?;
            if n == 0 {
                return Err(Error::NotFound(id.into()));
            }
            Ok(())
        })
    }

    // ---- Charges ----

    /// Charges of all subscriptions (archived ones left out) due in `[from, to]`.
    pub fn charges(&self, from: NaiveDate, to: NaiveDate, today: NaiveDate) -> Result<Vec<Charge>> {
        let mut out: Vec<Charge> = Vec::new();
        for s in self.subscriptions()?.into_iter().filter(|s| s.subscription.archived_at.is_none()) {
            out.extend(with_extras(occurrences(&s.periods, &s.overrides, from, to, today), &s.extras, from, to, today));
        }
        out.sort_by(|a, b| (a.due_date, &a.subscription_id).cmp(&(b.due_date, &b.subscription_id)));
        Ok(out)
    }

    /// The subscription's next charge on or after `today`.
    pub fn next_charge(&self, id: &str, today: NaiveDate) -> Result<Option<Charge>> {
        let s = self.subscription(id)?;
        Ok(next_charge(&s.periods, &s.overrides, today))
    }

    /// Last day of the billing cycle running on `today` (the default "ends
    /// on" for cancel): the day before the next charge, or `today` when no
    /// charge is coming.
    pub fn cycle_end(&self, id: &str, today: NaiveDate) -> Result<NaiveDate> {
        let s = self.subscription(id)?;
        // A charge today starts a new cycle; the one that matters is after it.
        let next = next_charge(&s.periods, &s.overrides, next_day(today));
        Ok(next.map_or(today, |c| prev_day(c.scheduled_date)))
    }
}
