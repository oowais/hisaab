//! ECB reference rates from the public Frankfurter API
//! (https://api.frankfurter.dev/v1/), cached in SQLite by the core. The
//! latest rates at most once a day; past ranges when a history needs them.
//! Offline or failing: keep what's cached (the UI says how old it is).

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::Deserialize;

const BASE: &str = "https://api.frankfurter.dev/v1";

/// One day's rates: currency → units per 1 EUR.
pub type DayRates = (NaiveDate, Vec<(String, f64)>);

#[derive(Deserialize)]
struct Latest {
    date: NaiveDate,
    rates: BTreeMap<String, f64>,
}

#[derive(Deserialize)]
struct Range {
    rates: BTreeMap<NaiveDate, BTreeMap<String, f64>>,
}

fn get<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .build()
        .into();
    agent
        .get(url)
        .call()
        .map_err(|e| format!("rates: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| format!("rates: {e}"))
}

/// The newest business day's rates.
pub fn latest() -> Result<DayRates, String> {
    let r: Latest = get(&format!("{BASE}/latest?base=EUR"))?;
    Ok((r.date, r.rates.into_iter().collect()))
}

/// Every business day's rates in `[from, to]`.
pub fn range(from: NaiveDate, to: NaiveDate) -> Result<Vec<DayRates>, String> {
    let r: Range = get(&format!("{BASE}/{from}..{to}?base=EUR"))?;
    Ok(r.rates.into_iter().map(|(d, m)| (d, m.into_iter().collect())).collect())
}
