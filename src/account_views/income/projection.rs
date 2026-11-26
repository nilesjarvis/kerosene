use crate::account_analytics::IncomeSnapshot;
use chrono::{Datelike, Months, NaiveDate};

// ---------------------------------------------------------------------------
// Income Projection Data
// ---------------------------------------------------------------------------

pub(super) fn projected_income_bars(data: &IncomeSnapshot, today: NaiveDate) -> Vec<(String, f64)> {
    let mut projection_bars = Vec::new();
    let Some(mut month_start) = today
        .with_day(1)
        .and_then(|date| date.checked_add_months(Months::new(1)))
    else {
        return projection_bars;
    };
    for _ in 0..12 {
        let Some(next_start) = month_start.checked_add_months(Months::new(1)) else {
            break;
        };
        let days = (next_start - month_start).num_days() as f64;
        let projected = data.net_yearly_projection * (days / 365.0);
        projection_bars.push((month_start.format("%b '%y").to_string(), projected));
        month_start = next_start;
    }
    projection_bars
}

#[cfg(test)]
mod tests {
    use super::projected_income_bars;
    use crate::account_analytics::IncomeSnapshot;
    use chrono::{TimeZone, Utc};

    #[test]
    fn projected_income_bars_use_supplied_month_as_anchor() {
        let snapshot = IncomeSnapshot {
            earned_total: 0.0,
            earned_24h: 0.0,
            earned_7d: 0.0,
            earned_30d: 0.0,
            net_yearly_projection: 365.0,
            current_supply_usd: 0.0,
            current_borrow_usd: 0.0,
            health: String::new(),
            health_factor: None,
            token_rows: Vec::new(),
            recent_hourly_payments: Vec::new(),
            invalid_token_rows: 0,
            invalid_interest_rows: 0,
        };
        let now = Utc
            .with_ymd_and_hms(2026, 12, 15, 12, 0, 0)
            .single()
            .expect("valid UTC timestamp");

        let bars = projected_income_bars(&snapshot, now.date_naive());

        assert_eq!(bars.len(), 12);
        assert_eq!(bars[0].0, "Jan '27");
        assert_eq!(bars[0].1, 31.0);
        assert_eq!(bars[1].0, "Feb '27");
        assert_eq!(bars[1].1, 28.0);
    }
}
