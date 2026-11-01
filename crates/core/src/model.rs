//! Domain types. A subscription is a stable identity; its terms live in
//! dated periods (trial / active / paused) that never overlap. Charges are
//! derived from periods, never stored; only exceptions are
//! (`ChargeOverride`). Money is integer minor units of `currency`.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

macro_rules! text_enum {
    ($(#[$m:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }

            pub fn parse(s: &str) -> Result<Self> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    _ => Err(Error::Invalid(format!("{}: unknown value {s:?}", stringify!($name)))),
                }
            }
        }
    };
}

text_enum!(
    /// What the payment is; only changes the default filter and wording.
    Kind { Subscription => "subscription", Bill => "bill" }
);
text_enum!(PeriodStatus { Trial => "trial", Active => "active", Paused => "paused" });
text_enum!(Unit { Day => "day", Week => "week", Month => "month", Year => "year" });
text_enum!(
    /// `Estimated`: variable bills (electricity); the amount is a guess.
    AmountKind { Fixed => "fixed", Estimated => "estimated" }
);
text_enum!(
    /// What differed for one charge. `Settled` only records what the bank
    /// actually took (any kind can carry settled amounts).
    OverrideKind { Skipped => "skipped", Moved => "moved", AmountChanged => "amount_changed", Settled => "settled" }
);
text_enum!(
    /// Shown status of a subscription (from its periods, see `status_on`).
    SubscriptionStatus { Trial => "trial", Active => "active", Paused => "paused", Ended => "ended" }
);
text_enum!(
    /// A charge counts as paid on its due date; later ones are upcoming.
    ChargeStatus { Upcoming => "upcoming", Paid => "paid" }
);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color: String,
    pub sort: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethod {
    pub id: String,
    pub name: String,
}

/// Editable identity of a subscription (never changes with price).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionFields {
    pub kind: Option<Kind>,
    pub name: String,
    pub template_key: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub category_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub url: Option<String>,
    pub notes: String,
    /// None = the global default.
    pub remind_days_before: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub template_key: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub category_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub url: Option<String>,
    pub notes: String,
    pub remind_days_before: Option<u32>,
    /// UTC ms.
    pub archived_at: Option<i64>,
}

/// The price side of a period: what is charged, how often.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Terms {
    pub amount_minor: i64,
    pub currency: String,
    pub every_n: u32,
    pub unit: Unit,
    pub plan_label: Option<String>,
    pub amount_kind: AmountKind,
}

/// A contiguous stretch with one set of terms. `end_date` None = open-ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    pub id: String,
    pub subscription_id: String,
    pub status: PeriodStatus,
    #[serde(flatten)]
    pub terms: Terms,
    /// First charge date of the schedule; charges repeat from it.
    pub anchor_date: NaiveDate,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
}

impl Period {
    pub fn covers(&self, day: NaiveDate) -> bool {
        self.start_date <= day && self.end_date.is_none_or(|e| day <= e)
    }

    /// Whether this period produces charges: paused never; a trial only when
    /// it costs something.
    pub fn charges(&self) -> bool {
        match self.status {
            PeriodStatus::Paused => false,
            PeriodStatus::Trial => self.terms.amount_minor > 0,
            PeriodStatus::Active => true,
        }
    }
}

/// One charge that differed from its period's rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargeOverride {
    pub id: String,
    pub period_id: String,
    /// The scheduled date this override is about.
    pub due_date: NaiveDate,
    pub kind: OverrideKind,
    pub new_due_date: Option<NaiveDate>,
    pub actual_amount_minor: Option<i64>,
    pub actual_currency: Option<String>,
    pub settled_amount_minor: Option<i64>,
    pub settled_currency: Option<String>,
}

/// A derived charge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Charge {
    pub subscription_id: String,
    pub period_id: String,
    pub due_date: NaiveDate,
    /// The date the rule gave (differs from `due_date` when moved).
    pub scheduled_date: NaiveDate,
    pub amount_minor: i64,
    pub currency: String,
    pub settled_amount_minor: Option<i64>,
    pub settled_currency: Option<String>,
    pub estimated: bool,
    pub status: ChargeStatus,
    pub is_override: bool,
}

/// A subscription with everything needed to derive its charges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDetail {
    #[serde(flatten)]
    pub subscription: Subscription,
    /// By start date.
    pub periods: Vec<Period>,
    pub overrides: Vec<ChargeOverride>,
}
