use crate::account::ClearinghouseState;
use crate::helpers::{format_decimal_with_commas, parse_number, positive_finite_value};

// ---------------------------------------------------------------------------
// Reduce-Only Position Sizing
// ---------------------------------------------------------------------------

pub(super) fn percentage_for_position_quantity(
    quantity: f64,
    position_size_coin: f64,
    quantity_is_usd: bool,
    reference_price: Option<f64>,
) -> f32 {
    let Some(quantity) = positive_finite_value(quantity) else {
        return 0.0;
    };
    let Some(position_size_coin) = positive_finite_value(position_size_coin) else {
        return 0.0;
    };

    let max_quantity = if quantity_is_usd {
        let Some(reference_price) = reference_price.and_then(positive_finite_value) else {
            return 0.0;
        };
        position_size_coin * reference_price
    } else {
        position_size_coin
    };

    let Some(max_quantity) = positive_finite_value(max_quantity) else {
        return 0.0;
    };

    (((quantity / max_quantity) * 100.0) as f32).clamp(0.0, 100.0)
}

pub(super) fn position_quantity_for_percentage(
    percentage: f32,
    position_size_coin: f64,
    quantity_is_usd: bool,
    reference_price: Option<f64>,
    decimals: usize,
) -> String {
    let Some(position_size_coin) = positive_finite_value(position_size_coin) else {
        return "0".to_string();
    };
    if !percentage.is_finite() {
        return "0".to_string();
    }

    let target_coin = position_size_coin * (percentage.clamp(0.0, 100.0) as f64 / 100.0);
    if quantity_is_usd {
        if let Some(reference_price) = reference_price.and_then(positive_finite_value) {
            return format_decimal_with_commas(target_coin * reference_price, 2);
        }
        "0".to_string()
    } else {
        format_decimal_with_commas(target_coin, decimals)
    }
}

pub(in crate::order_update::form) fn position_size_for_symbol(
    clearinghouse: &ClearinghouseState,
    active_symbol: &str,
) -> Option<f64> {
    let asset_position = clearinghouse
        .asset_positions
        .iter()
        .find(|asset_position| asset_position.position.coin == active_symbol)
        .or_else(|| {
            clearinghouse.asset_positions.iter().find(|asset_position| {
                position_coin_matches(&asset_position.position.coin, active_symbol)
            })
        })?;

    parse_number(&asset_position.position.szi)
        .map(f64::abs)
        .and_then(positive_finite_value)
}

fn position_coin_matches(position_coin: &str, active_symbol: &str) -> bool {
    if position_coin == active_symbol {
        return true;
    }

    match (position_coin.split_once(':'), active_symbol.split_once(':')) {
        (None, Some((_, active_suffix))) => position_coin == active_suffix,
        _ => false,
    }
}

#[cfg(test)]
mod tests;
