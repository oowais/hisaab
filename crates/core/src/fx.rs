//! The local FX rate cache (`fx_rate`, not synced) and conversion of
//! charges into the main currency. Fetching from Frankfurter is the app's
//! job (milestone 3); this only stores and reads rates.

use chrono::NaiveDate;
use rusqlite::{params, OptionalExtension};

use crate::currency::{convert, valuation, RateWhen};
use crate::model::Charge;
use crate::store::Store;
use crate::Result;

/// Rates older than this many days are shown as stale.
pub const STALE_DAYS: i64 = 3;

impl Store {
    /// Cache one day's rates (units per 1 EUR), replacing that day's values.
    pub fn put_rates(&mut self, date: NaiveDate, rates: &[(String, f64)]) -> Result<()> {
        self.write(|c, _| {
            let mut st = c.prepare(
                "INSERT INTO fx_rate (date, currency, rate_vs_eur) VALUES (?1, ?2, ?3)
                 ON CONFLICT(date, currency) DO UPDATE SET rate_vs_eur = excluded.rate_vs_eur",
            )?;
            for (cur, rate) in rates.iter().filter(|(_, r)| r.is_finite() && *r > 0.0) {
                st.execute(params![date.to_string(), cur, rate])?;
            }
            Ok(())
        })
    }

    /// Rate of `currency` per 1 EUR: on `when` (newest on or before that
    /// day) or the latest. EUR is always 1. None when not cached (or not an
    /// ECB currency).
    pub fn rate(&self, currency: &str, when: RateWhen) -> Result<Option<f64>> {
        if currency == "EUR" {
            return Ok(Some(1.0));
        }
        let rate = match when {
            RateWhen::On(d) => self.conn.query_row(
                "SELECT rate_vs_eur FROM fx_rate WHERE currency = ?1 AND date <= ?2 ORDER BY date DESC LIMIT 1",
                params![currency, d.to_string()],
                |r| r.get(0),
            ),
            RateWhen::Latest => self.conn.query_row(
                "SELECT rate_vs_eur FROM fx_rate WHERE currency = ?1 ORDER BY date DESC LIMIT 1",
                [currency],
                |r| r.get(0),
            ),
        };
        Ok(rate.optional()?)
    }

    /// Day of the newest cached rates.
    pub fn latest_rates_date(&self) -> Result<Option<NaiveDate>> {
        let d: Option<String> = self.conn.query_row("SELECT MAX(date) FROM fx_rate", [], |r| r.get(0))?;
        Ok(d.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()))
    }

    /// Day of the oldest cached rates.
    pub fn earliest_rates_date(&self) -> Result<Option<NaiveDate>> {
        let d: Option<String> = self.conn.query_row("SELECT MIN(date) FROM fx_rate", [], |r| r.get(0))?;
        Ok(d.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()))
    }

    /// Whether to show "rates from <date>": none cached, or older than 3 days.
    pub fn rates_stale(&self, today: NaiveDate) -> Result<bool> {
        Ok(self.latest_rates_date()?.is_none_or(|d| (today - d).num_days() > STALE_DAYS))
    }

    /// `charge` in minor units of `main` (rules in currency.rs). None when a
    /// rate is missing: show the original only and leave it out of totals.
    pub fn charge_in(&self, charge: &Charge, main: &str) -> Result<Option<i64>> {
        let (amount, cur, when) = valuation(charge);
        if cur == main {
            return Ok(Some(amount));
        }
        let (Some(from), Some(to)) = (self.rate(cur, when)?, self.rate(main, when)?) else {
            return Ok(None);
        };
        Ok(Some(convert(amount, cur, main, from, to)))
    }

    /// `amount_minor` of `currency` in `main` at the latest rates (headers,
    /// forecasts). None when a rate is missing.
    pub fn amount_in(&self, amount_minor: i64, currency: &str, main: &str) -> Result<Option<i64>> {
        if currency == main {
            return Ok(Some(amount_minor));
        }
        let (Some(from), Some(to)) = (self.rate(currency, RateWhen::Latest)?, self.rate(main, RateWhen::Latest)?) else {
            return Ok(None);
        };
        Ok(Some(convert(amount_minor, currency, main, from, to)))
    }
}
