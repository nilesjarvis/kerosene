use crate::account::SpotBalance;
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
        .filter(|price| price.is_finite() && *price > 0.0);

    if let Some(price) = mid {
        Some(total * price)
    } else {
        parse_spot_number(&balance.entry_ntl).filter(|v| *v > 0.0)
    }
}

fn parse_spot_number(value: &str) -> Option<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
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
mod tests {
    use super::*;

    fn balance(coin: &str, total: &str, entry_ntl: &str) -> SpotBalance {
        SpotBalance {
            coin: coin.to_string(),
            token: None,
            total: total.to_string(),
            hold: "0".to_string(),
            entry_ntl: entry_ntl.to_string(),
            supplied: None,
        }
    }

    #[test]
    fn spot_equity_estimate_rejects_invalid_balance_numbers() {
        let mids = HashMap::new();
        assert_eq!(
            estimate_spot_equity(&[balance("USDC", "10", "0")], &mids),
            Some(10.0)
        );
        assert_eq!(
            estimate_spot_equity(&[balance("USDC", "bad", "0")], &mids),
            None
        );
        assert_eq!(
            estimate_spot_equity(&[balance("USDC", "NaN", "0")], &mids),
            None
        );
    }

    #[test]
    fn spot_equity_estimate_requires_mid_or_valid_entry_notional_for_non_stables() {
        let mut mids = HashMap::new();
        mids.insert("PURR".to_string(), 4.0);
        assert_eq!(
            estimate_spot_equity(&[balance("PURR", "2", "3")], &mids),
            Some(8.0)
        );

        let mids = HashMap::new();
        assert_eq!(
            estimate_spot_equity(&[balance("PURR", "2", "3")], &mids),
            Some(3.0)
        );
        assert_eq!(
            estimate_spot_equity(&[balance("PURR", "2", "bad")], &mids),
            None
        );
    }

    #[test]
    fn outcome_balance_uses_trade_coin_mid() {
        let mut mids = HashMap::new();
        mids.insert("#650".to_string(), 0.42);

        assert_eq!(
            estimate_spot_equity(&[balance("+650", "10", "1")], &mids),
            Some(4.2)
        );
    }
}
