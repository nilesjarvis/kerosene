use crate::account_analytics::IncomeSnapshot;
use chrono::{Datelike, TimeZone, Utc};

// ---------------------------------------------------------------------------
// Income Projection Data
// ---------------------------------------------------------------------------

pub(super) fn projected_income_bars(data: &IncomeSnapshot) -> Vec<(String, f64)> {
    let now = Utc::now();
    let mut projection_bars = Vec::new();
    let mut year = now.year();
    let mut month = now.month();
    for _ in 0..12 {
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
        let month_start = Utc
            .with_ymd_and_hms(year, month, 1, 0, 0, 0)
            .single()
            .unwrap_or_else(Utc::now);
        let (next_year, next_month) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        let next_start = Utc
            .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
            .single()
            .unwrap_or_else(Utc::now);
        let days = (next_start - month_start).num_days().max(28) as f64;
        let projected = data.net_yearly_projection * (days / 365.0);
        projection_bars.push((month_start.format("%b '%y").to_string(), projected));
    }
    projection_bars
}
