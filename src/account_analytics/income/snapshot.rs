use super::parsing::{income_per_day_dedup_with_stats, parse_f64_str};
use crate::account_analytics::model::{
    BorrowLendInterestEntry, BorrowLendReserveState, BorrowLendTokenState, BorrowLendUserState,
    IncomeHourlyPayment, IncomeSnapshot, IncomeTokenRow,
};
use crate::app_time::now_ms;

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

        let Some(row) = income_token_row(*token, state, reserve, token_name_by_id) else {
            invalid_token_rows += 1;
            continue;
        };

        net_yearly_projection += row.net_yearly_usd;
        current_supply_usd += row.supply_usd;
        current_borrow_usd += row.borrow_usd;
        token_rows.push(row);
    }

    token_rows.sort_by(|a, b| {
        b.net_yearly_usd
            .abs()
            .partial_cmp(&a.net_yearly_usd.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let dedup_by_day = income_per_day_dedup_with_stats(interest_entries);
    let earned_total: f64 = dedup_by_day.values.values().sum();
    let recent_hourly = recent_hourly_payments(
        interest_entries,
        &user_state,
        reserve_by_token,
        token_name_by_id,
    );

    let now_ms = now_ms();
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

fn income_token_row(
    token: u32,
    state: &BorrowLendTokenState,
    reserve: &BorrowLendReserveState,
    token_name_by_id: &HashMap<u32, String>,
) -> Option<IncomeTokenRow> {
    let px = parse_f64_str(&reserve.oracle_px)?;
    let supply_rate = parse_f64_str(&reserve.supply_yearly_rate)?;
    let borrow_rate = parse_f64_str(&reserve.borrow_yearly_rate)?;
    let supply_value = parse_f64_str(&state.supply.value)?;
    let borrow_value = parse_f64_str(&state.borrow.value)?;
    let supply_usd = supply_value * px;
    let borrow_usd = borrow_value * px;
    let net_yearly = supply_usd * supply_rate - borrow_usd * borrow_rate;
    if !supply_usd.is_finite() || !borrow_usd.is_finite() || !net_yearly.is_finite() {
        return None;
    }

    Some(IncomeTokenRow {
        token,
        token_label: token_name_by_id
            .get(&token)
            .cloned()
            .unwrap_or_else(|| format!("#{token}")),
        supply_usd,
        borrow_usd,
        supply_rate,
        net_yearly_usd: net_yearly,
    })
}

/// Hours of interest accrual per year, used to annualize one hourly payment.
const HOURLY_ACCRUALS_PER_YEAR: f64 = 365.0 * 24.0;

fn recent_hourly_payments(
    interest_entries: &[BorrowLendInterestEntry],
    user_state: &BorrowLendUserState,
    reserve_by_token: &HashMap<u32, BorrowLendReserveState>,
    token_name_by_id: &HashMap<u32, String>,
) -> Vec<IncomeHourlyPayment> {
    let name_to_id: HashMap<&str, u32> = token_name_by_id
        .iter()
        .map(|(id, name)| (name.as_str(), *id))
        .collect();
    let supplied_value_by_token: HashMap<u32, f64> = user_state
        .token_to_state
        .iter()
        .filter_map(|(token, state)| {
            parse_f64_str(&state.supply.value).map(|value| (*token, value))
        })
        .collect();
    let mut hourly_entries: Vec<_> = interest_entries
        .iter()
        .filter(|e| e.n_samples.is_none())
        .collect();
    hourly_entries.sort_by_key(|entry| Reverse(entry.time));

    hourly_entries
        .into_iter()
        .filter_map(|e| {
            let supply = parse_f64_str(&e.supply)?;
            let borrow = parse_f64_str(&e.borrow)?;
            let net = supply - borrow;
            if !net.is_finite() {
                return None;
            }
            let token_id = e.token.parse::<u32>().ok();
            let token_label = token_id
                .and_then(|idx| token_name_by_id.get(&idx).cloned())
                .unwrap_or_else(|| e.token.clone());
            let supply_rate = token_id
                .or_else(|| name_to_id.get(e.token.as_str()).copied())
                .and_then(|idx| {
                    payment_supply_rate(supply, idx, &supplied_value_by_token, reserve_by_token)
                })
                .unwrap_or(0.0);
            Some(IncomeHourlyPayment {
                time: e.time,
                token_label,
                supply,
                borrow,
                supply_rate,
                net,
            })
        })
        .take(12)
        .collect()
}

/// Supply APR that applied when one hourly payment accrued, recovered from
/// the payment against the position it was earned on (`interest * price /
/// supplied value`, annualized). Reserve rates change every hour, so this
/// varies per payment; falls back to the reserve's current supply rate when
/// the position has since been closed or its value or price is unusable, and
/// to 0 when the token has no reserve state.
fn payment_supply_rate(
    hourly_supply: f64,
    token: u32,
    supplied_value_by_token: &HashMap<u32, f64>,
    reserve_by_token: &HashMap<u32, BorrowLendReserveState>,
) -> Option<f64> {
    let reserve = reserve_by_token.get(&token)?;
    let fallback = parse_f64_str(&reserve.supply_yearly_rate).unwrap_or(0.0);
    let supplied_value = supplied_value_by_token.get(&token).copied().unwrap_or(0.0);
    let oracle_px = parse_f64_str(&reserve.oracle_px).unwrap_or(0.0);
    if supplied_value <= 0.0 || oracle_px <= 0.0 {
        return Some(fallback);
    }
    let rate = hourly_supply * oracle_px / supplied_value * HOURLY_ACCRUALS_PER_YEAR;
    Some(if rate.is_finite() { rate } else { fallback })
}

#[cfg(test)]
mod tests;
