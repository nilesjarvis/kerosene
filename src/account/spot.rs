use crate::account::SpotBalance;
use crate::api::{ExchangeSymbol, MarketType};
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
    if total.abs() <= f64::EPSILON {
        return Some(0.0);
    }

    if is_stable(&balance.coin) {
        return Some(total);
    }

    let mid = mid_candidates(&balance.coin)
        .into_iter()
        .find_map(|key| mids.get(&key).copied())
        .and_then(positive_finite_value);

    mid.map(|price| total * price)
}

fn parse_spot_number(value: &str) -> Option<f64> {
    parse_finite_number(value)
}

fn is_stable(coin: &str) -> bool {
    matches!(coin, "USDC" | "USDE" | "USDT0" | "USDH")
}

fn mid_candidates(coin: &str) -> Vec<String> {
    let mut out = vec![spot_balance_mid_key(coin)];

    if let Some(encoding) = coin.strip_prefix('+') {
        out.push(format!("#{encoding}"));
    }

    // The API names canonical spot pairs directly (PURR/USDC is the only one
    // today), so "{coin}/USDC" is a real allMids key for those tokens.
    out.push(format!("{coin}/USDC"));

    out.sort();
    out.dedup();
    out
}

fn spot_balance_mid_key(coin: &str) -> String {
    format!("__kerosene_spot_balance__:{}", coin.to_ascii_uppercase())
}

/// Add balance-token-specific marks using validated spot metadata. Raw
/// `allMids` also contains bare perpetual tickers; consumers must never use a
/// same-ticker perp to value a spot balance. USD-stable quote pairs are ranked
/// deterministically, preferring USDC when duplicate base markets exist.
pub(super) fn augment_spot_balance_mids(
    mids: &mut HashMap<String, f64>,
    symbols: &[ExchangeSymbol],
) {
    let mut selected: HashMap<String, (u8, u32, f64)> = HashMap::new();
    for symbol in symbols.iter().filter(|symbol| {
        symbol.market_type == MarketType::Spot && symbol.spot_quote_is_usd_stable()
    }) {
        let Some(mid) = mids
            .get(&symbol.key)
            .copied()
            .and_then(positive_finite_value)
        else {
            continue;
        };
        let quote_rank = symbol
            .display_name
            .as_deref()
            .and_then(|display| display.rsplit_once('/'))
            .map(|(_, quote)| match quote {
                "USDC" => 0,
                "USDH" => 1,
                "USDT0" => 2,
                "USDE" => 3,
                _ => u8::MAX,
            })
            .unwrap_or(u8::MAX);
        let key = symbol.ticker.to_ascii_uppercase();
        let candidate = (quote_rank, symbol.asset_index, mid);
        if selected
            .get(&key)
            .is_none_or(|current| candidate < *current)
        {
            selected.insert(key, candidate);
        }
    }

    for (coin, (_, _, mid)) in selected {
        mids.insert(spot_balance_mid_key(&coin), mid);
    }
}

#[cfg(test)]
mod tests;
