//! Search: payments whose name, notes, link, a plan, the category or the
//! payment method contain the text; running, ended and archived.

use chrono::NaiveDate;

use crate::model::SubscriptionStatus;
use crate::overview::SubscriptionView;
use crate::store::Store;
use crate::Result;

/// At most this many results.
const LIMIT: u32 = 50;

impl Store {
    /// Subscriptions matching `query` (case-insensitive, every word must
    /// appear somewhere). Running ones first, then ended, then archived;
    /// by name within each. Deleted ones never.
    pub fn search(&self, query: &str, today: NaiveDate) -> Result<Vec<SubscriptionView>> {
        let words: Vec<String> = query.split_whitespace().map(like_pattern).collect();
        if words.is_empty() {
            return Ok(vec![]);
        }
        let conds = (1..=words.len())
            .map(|i| {
                format!(
                    "(s.name LIKE ?{i} ESCAPE '\\' OR s.notes LIKE ?{i} ESCAPE '\\' OR s.url LIKE ?{i} ESCAPE '\\'
                      OR EXISTS (SELECT 1 FROM period p WHERE p.subscription_id = s.id AND p.deleted_at IS NULL
                                 AND p.plan_label LIKE ?{i} ESCAPE '\\')
                      OR EXISTS (SELECT 1 FROM category c WHERE c.id = s.category_id AND c.deleted_at IS NULL
                                 AND c.name LIKE ?{i} ESCAPE '\\')
                      OR EXISTS (SELECT 1 FROM payment_method m WHERE m.id = s.payment_method_id AND m.deleted_at IS NULL
                                 AND m.name LIKE ?{i} ESCAPE '\\'))"
                )
            })
            .collect::<Vec<_>>()
            .join(" AND ");
        let sql = format!(
            "SELECT s.id FROM subscription s WHERE s.deleted_at IS NULL AND {conds}
             ORDER BY s.name COLLATE NOCASE, s.id LIMIT {LIMIT}"
        );
        let ids = {
            let mut st = self.conn.prepare(&sql)?;
            let rows = st.query_map(rusqlite::params_from_iter(words.iter()), |r| r.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let main = self.main_currency()?;
        let mut views = ids
            .iter()
            .map(|id| self.view(&self.subscription(id)?, today, &main))
            .collect::<Result<Vec<_>>>()?;
        // Stable: keeps name order within each group.
        views.sort_by_key(|v| match (v.subscription.archived_at, v.status) {
            (Some(_), _) => 2,
            (None, SubscriptionStatus::Ended) => 1,
            _ => 0,
        });
        Ok(views)
    }
}

/// `%word%`, with LIKE's own wildcards taken literally.
fn like_pattern(word: &str) -> String {
    let mut p = String::from("%");
    for ch in word.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            p.push('\\');
        }
        p.push(ch);
    }
    p.push('%');
    p
}
