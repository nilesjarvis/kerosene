use super::parsing::{income_per_day_dedup_with_stats, parse_f64_str};
use crate::account_analytics::model::{
    BorrowLendInterestEntry, BorrowLendReserveState, BorrowLendUserState, IncomeHourlyPayment,
    IncomeSnapshot, IncomeTokenRow,
};

use std::cmp::Reverse;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Income Snapshot Assembly
// ---------------------------------------------------------------------------

pub(super) fn build_income_snapshot(
    user_state: BorrowLendUserState,
    interest_entries: &[BorrowLendInterestEntry],
    reserve_by_token: &HashMap<u32, BorrowLendReserveState>,
    token_name_by_id: &HashMap<u32, String>,
) -> IncomeSnapshot {
    let mut token_rows: Vec<IncomeTokenRow> = Vec::new();
    let mut net_yearly_projection = 0.0;
    let mut current_supply_usd = 0.0;
    let mut current_borrow_usd = 0.0;
    let mut invalid_token_rows = 0_usize;

    for (token, state) in &user_state.token_to_state {
        let Some(reserve) = reserve_by_token.get(token) else {
            continue;
        };

        let Some(px) = parse_f64_str(&reserve.oracle_px) else {
            invalid_token_rows += 1;
            continue;
        };
        let Some(supply_rate) = parse_f64_str(&reserve.supply_yearly_rate) else {
            invalid_token_rows += 1;
            continue;
        };
        let Some(borrow_rate) = parse_f64_str(&reserve.borrow_yearly_rate) else {
            invalid_token_rows += 1;
            continue;
        };

        let Some(supply_value) = parse_f64_str(&state.supply.value) else {
            invalid_token_rows += 1;
            continue;
        };
        let Some(borrow_value) = parse_f64_str(&state.borrow.value) else {
            invalid_token_rows += 1;
            continue;
        };
        let supply_usd = supply_value * px;
        let borrow_usd = borrow_value * px;

        let net_yearly = supply_usd * supply_rate - borrow_usd * borrow_rate;
        if !supply_usd.is_finite() || !borrow_usd.is_finite() || !net_yearly.is_finite() {
            invalid_token_rows += 1;
            continue;
        }

        token_rows.push(IncomeTokenRow {
            token: *token,
            token_label: token_name_by_id
                .get(token)
                .cloned()
                .unwrap_or_else(|| format!("#{token}")),
            supply_usd,
            borrow_usd,
            supply_rate,
            net_yearly_usd: net_yearly,
        });

        net_yearly_projection += net_yearly;
        current_supply_usd += supply_usd;
        current_borrow_usd += borrow_usd;
    }

    token_rows.sort_by(|a, b| {
        b.net_yearly_usd
            .abs()
            .partial_cmp(&a.net_yearly_usd.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let dedup_by_day = income_per_day_dedup_with_stats(interest_entries);
    let earned_total: f64 = dedup_by_day.values.values().sum();
    let recent_hourly = recent_hourly_payments(interest_entries, token_name_by_id);

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let day_ms: u64 = 24 * 60 * 60 * 1000;

    let sum_since_days = |days: u64| -> f64 {
        let cutoff = now_ms.saturating_sub(days.saturating_mul(day_ms));
        dedup_by_day
            .values
            .iter()
            .filter(|(day, _)| **day >= cutoff)
            .map(|(_, v)| *v)
            .sum()
    };

    IncomeSnapshot {
        earned_total,
        earned_24h: sum_since_days(1),
        earned_7d: sum_since_days(7),
        earned_30d: sum_since_days(30),
        net_yearly_projection,
        current_supply_usd,
        current_borrow_usd,
        health: user_state.health,
        health_factor: user_state.health_factor,
        token_rows,
        recent_hourly_payments: recent_hourly,
        invalid_token_rows,
        invalid_interest_rows: dedup_by_day.invalid_rows,
    }
}

fn recent_hourly_payments(
    interest_entries: &[BorrowLendInterestEntry],
    token_name_by_id: &HashMap<u32, String>,
) -> Vec<IncomeHourlyPayment> {
    let mut recent_hourly: Vec<IncomeHourlyPayment> = interest_entries
        .iter()
        .filter(|e| e.n_samples.is_none())
        .filter_map(|e| {
            let supply = parse_f64_str(&e.supply)?;
            let borrow = parse_f64_str(&e.borrow)?;
            let net = supply - borrow;
            if !net.is_finite() {
                return None;
            }
            let token_label = e
                .token
                .parse::<u32>()
                .ok()
                .and_then(|idx| token_name_by_id.get(&idx).cloned())
                .unwrap_or_else(|| e.token.clone());
            Some(IncomeHourlyPayment {
                time: e.time,
                token_label,
                supply,
                borrow,
                net,
            })
        })
        .collect();
    recent_hourly.sort_by_key(|payment| Reverse(payment.time));
    recent_hourly.truncate(12);
    recent_hourly
}

#[cfg(test)]
mod tests {
    use super::build_income_snapshot;
    use crate::account_analytics::model::{
        BorrowLendInterestEntry, BorrowLendReserveState, BorrowLendSideState, BorrowLendTokenState,
        BorrowLendUserState,
    };

    use std::collections::HashMap;

    fn reserve(
        oracle_px: &str,
        supply_yearly_rate: &str,
        borrow_yearly_rate: &str,
    ) -> BorrowLendReserveState {
        BorrowLendReserveState {
            borrow_yearly_rate: borrow_yearly_rate.to_string(),
            supply_yearly_rate: supply_yearly_rate.to_string(),
            oracle_px: oracle_px.to_string(),
        }
    }

    fn side(value: &str) -> BorrowLendSideState {
        BorrowLendSideState {
            value: value.to_string(),
        }
    }

    fn token_state(supply: &str, borrow: &str) -> BorrowLendTokenState {
        BorrowLendTokenState {
            supply: side(supply),
            borrow: side(borrow),
        }
    }

    fn interest(time: u64, token: &str, supply: &str, borrow: &str) -> BorrowLendInterestEntry {
        BorrowLendInterestEntry {
            time,
            token: token.to_string(),
            borrow: borrow.to_string(),
            supply: supply.to_string(),
            n_samples: None,
        }
    }

    #[test]
    fn income_snapshot_skips_invalid_numeric_rows_and_reports_counts() {
        let user_state = BorrowLendUserState {
            token_to_state: vec![(0, token_state("10", "4")), (1, token_state("2", "0"))],
            health: "healthy".to_string(),
            health_factor: Some("10".to_string()),
        };
        let reserve_by_token = HashMap::from([
            (0, reserve("2", "0.10", "0.20")),
            (1, reserve("bad", "0.10", "0.20")),
        ]);
        let token_name_by_id = HashMap::from([(0, "USDC".to_string()), (1, "BAD".to_string())]);
        let interest_entries = vec![
            interest(1_000, "0", "5", "1"),
            interest(2_000, "0", "bad", "2"),
            interest(3_000, "0", "NaN", "0"),
        ];

        let snapshot = build_income_snapshot(
            user_state,
            &interest_entries,
            &reserve_by_token,
            &token_name_by_id,
        );

        assert_eq!(snapshot.token_rows.len(), 1);
        assert_eq!(snapshot.current_supply_usd, 20.0);
        assert_eq!(snapshot.current_borrow_usd, 8.0);
        assert!((snapshot.net_yearly_projection - 0.4).abs() < 1e-12);
        assert_eq!(snapshot.earned_total, 4.0);
        assert_eq!(snapshot.recent_hourly_payments.len(), 1);
        assert_eq!(snapshot.invalid_token_rows, 1);
        assert_eq!(snapshot.invalid_interest_rows, 2);
    }
}
