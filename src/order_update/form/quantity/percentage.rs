// ---------------------------------------------------------------------------
// Percentage Quantity Math
// ---------------------------------------------------------------------------

use crate::helpers::{format_decimal_with_commas, positive_finite_value};

pub(in crate::order_update::form) fn order_percentage_for_quantity(
    quantity: f64,
    quantity_is_usd: bool,
    reference_price: Option<f64>,
    max_notional: f64,
) -> f32 {
    let Some(quantity) = positive_finite_value(quantity) else {
        return 0.0;
    };
    let Some(max_notional) = positive_finite_value(max_notional) else {
        return 0.0;
    };

    let target_notional = if quantity_is_usd {
        quantity
    } else if let Some(reference_price) = reference_price.and_then(positive_finite_value) {
        quantity * reference_price
    } else {
        return 0.0;
    };

    if !target_notional.is_finite() {
        return 0.0;
    }

    (((target_notional / max_notional) * 100.0) as f32).clamp(0.0, 100.0)
}

pub(in crate::order_update::form) fn quantity_for_percentage(
    percentage: f32,
    max_notional: f64,
    quantity_is_usd: bool,
    reference_price: Option<f64>,
    decimals: usize,
) -> String {
    if !percentage.is_finite() {
        return "0".to_string();
    }
    let Some(max_notional) = positive_finite_value(max_notional) else {
        return "0".to_string();
    };

    let target_notional = max_notional * (percentage.clamp(0.0, 100.0) as f64 / 100.0);
    if quantity_is_usd {
        return format_decimal_with_commas(target_notional, 2);
    }

    if let Some(reference_price) = reference_price.and_then(positive_finite_value) {
        let target_coin = target_notional / reference_price;
        format_decimal_with_commas(target_coin, decimals)
    } else {
        "0".to_string()
    }
}
