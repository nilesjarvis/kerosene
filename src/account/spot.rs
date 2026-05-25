use crate::account::SpotBalance;
use crate::helpers::{parse_finite_number, positive_finite_value};

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Spot Equity Estimation
// ---------------------------------------------------------------------------

pub(super) fn estimate_spot_equity(
    balances: &[SpotBalance],
    mids: &HashMap<String, f64>,
) -> Option<f64> {
    let mut total = 0.0;
    for balance in balances {
        total += estimate_balance_value(balance, mids)?;
    }
    Some(total)
}

fn estimate_balance_value(balance: &SpotBalance, mids: &HashMap<String, f64>) -> Option<f64> {
    let total = parse_spot_number(&balance.total)?;
    if total <= 0.0 {
        return Some(0.0);
    }

    if is_stable(&balance.coin) {
        return Some(total);
    }

    let mid = mid_candidates(&balance.coin)
        .into_iter()
        .find_map(|key| mids.get(&key).copied())
        .and_then(positive_finite_value);

    if let Some(price) = mid {
        Some(total * price)
    } else {
        parse_spot_number(&balance.entry_ntl).and_then(positive_finite_value)
    }
}

fn parse_spot_number(value: &str) -> Option<f64> {
    parse_finite_number(value)
}

fn is_stable(coin: &str) -> bool {
    matches!(coin, "USDC" | "USDE" | "USDT0" | "USDH")
}

fn mid_candidates(coin: &str) -> Vec<String> {
    let mut out = vec![coin.to_string()];

    if let Some(encoding) = coin.strip_prefix('+') {
        out.push(format!("#{encoding}"));
    }

    if let Some(stripped) = coin.strip_prefix('U') {
        out.push(stripped.to_string());
    }

    // Strip numeric suffixes (e.g., XMR1 -> XMR).
    let stripped_num = coin.trim_end_matches(|c: char| c.is_ascii_digit());
    if stripped_num != coin && !stripped_num.is_empty() {
        out.push(stripped_num.to_string());
        if let Some(stripped) = stripped_num.strip_prefix('U') {
            out.push(stripped.to_string());
        }
    }

    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests;
