//! Currencies and conversion. Rates are units of a currency per 1 EUR (the
//! ECB reference rates Frankfurter serves); A→B goes through EUR. Converted
//! values are never stored, only computed for display and totals.
//!
//! Which rate a charge uses:
//! - past charge with a settled amount → that amount (converted on its due
//!   date if it's in another currency);
//! - other past charge → the rate on its due date;
//! - upcoming charge, forecasts, headers → the latest cached rate.
//!
//! "On a date" = the newest rate on or before it, so weekends and holidays
//! use the previous business day.

use chrono::NaiveDate;

use crate::model::{Charge, ChargeStatus};

/// Active ISO 4217 codes.
const CODES: &str = "AED AFN ALL AMD ANG AOA ARS AUD AWG AZN BAM BBD BDT BGN BHD BIF BMD BND BOB BRL BSD BTN BWP \
    BYN BZD CAD CDF CHF CLP CNY COP CRC CUP CVE CZK DJF DKK DOP DZD EGP ERN ETB EUR FJD FKP GBP GEL GHS GIP GMD \
    GNF GTQ GYD HKD HNL HTG HUF IDR ILS INR IQD IRR ISK JMD JOD JPY KES KGS KHR KMF KPW KRW KWD KYD KZT LAK LBP \
    LKR LRD LSL LYD MAD MDL MGA MKD MMK MNT MOP MRU MUR MVR MWK MXN MYR MZN NAD NGN NIO NOK NPR NZD OMR PAB PEN \
    PGK PHP PKR PLN PYG QAR RON RSD RUB RWF SAR SBD SCR SDG SEK SGD SHP SLE SOS SRD SSP STN SVC SYP SZL THB TJS \
    TMT TND TOP TRY TTD TWD TZS UAH UGX USD UYU UZS VES VND VUV WST XAF XCD XCG XOF XPF YER ZAR ZMW ZWG";

/// Currencies without minor units, and with three (ISO 4217).
const ZERO_DIGITS: &[&str] = &[
    "BIF", "CLP", "DJF", "GNF", "ISK", "JPY", "KMF", "KRW", "PYG", "RWF", "UGX", "VND", "VUV", "XAF", "XOF", "XPF",
];
const THREE_DIGITS: &[&str] = &["BHD", "IQD", "JOD", "KWD", "LYD", "OMR", "TND"];

pub fn is_currency_code(code: &str) -> bool {
    code.len() == 3 && CODES.split_ascii_whitespace().any(|c| c == code)
}

/// Decimal places of the currency's minor unit (JPY 0, EUR 2, KWD 3).
pub fn minor_digits(code: &str) -> u32 {
    if ZERO_DIGITS.contains(&code) {
        0
    } else if THREE_DIGITS.contains(&code) {
        3
    } else {
        2
    }
}

/// `amount_minor` of `from` in minor units of `to`, given each currency's
/// rate per 1 EUR. Rounds half away from zero.
pub fn convert(amount_minor: i64, from: &str, to: &str, from_per_eur: f64, to_per_eur: f64) -> i64 {
    if from == to {
        return amount_minor;
    }
    let major = amount_minor as f64 / 10f64.powi(minor_digits(from) as i32);
    let converted = major / from_per_eur * to_per_eur;
    (converted * 10f64.powi(minor_digits(to) as i32)).round() as i64
}

/// Which rate to value a charge with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateWhen {
    On(NaiveDate),
    Latest,
}

/// The amount and currency to value `charge` from, and which rate to use.
pub fn valuation(charge: &Charge) -> (i64, &str, RateWhen) {
    match (charge.status, charge.settled_amount_minor, charge.settled_currency.as_deref()) {
        (ChargeStatus::Paid, Some(a), Some(c)) => (a, c, RateWhen::On(charge.due_date)),
        (ChargeStatus::Paid, _, _) => (charge.amount_minor, &charge.currency, RateWhen::On(charge.due_date)),
        (ChargeStatus::Upcoming, _, _) => (charge.amount_minor, &charge.currency, RateWhen::Latest),
    }
}
