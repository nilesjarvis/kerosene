use crate::helpers::{format_decimal_with_commas, parse_positive_number, positive_finite_value};

// ---------------------------------------------------------------------------
// Quick Order Calculations
// ---------------------------------------------------------------------------

pub(super) fn parse_positive_finite(value: &str) -> Option<f64> {
    parse_positive_number(value)
}

pub(super) fn quick_order_quantity_for_percentage(
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

pub(super) fn toggled_quick_order_quantity_text(
    quantity: &str,
    target_is_usd: bool,
    reference_price: Option<f64>,
    decimals: usize,
) -> String {
    let Some(quantity) = parse_positive_finite(quantity) else {
        return quantity.to_string();
    };
    let Some(reference_price) = reference_price.and_then(positive_finite_value) else {
        return quantity.to_string();
    };

    if target_is_usd {
        format_decimal_with_commas(quantity * reference_price, 2)
    } else {
        format_decimal_with_commas(quantity / reference_price, decimals)
    }
}
