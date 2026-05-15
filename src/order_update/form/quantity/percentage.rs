// ---------------------------------------------------------------------------
// Percentage Quantity Math
// ---------------------------------------------------------------------------

use crate::helpers::format_decimal_with_commas;

pub(in crate::order_update::form) fn order_percentage_for_quantity(
    quantity: f64,
    quantity_is_usd: bool,
    reference_price: Option<f64>,
    max_notional: f64,
) -> f32 {
    if !quantity.is_finite() || quantity <= 0.0 || !max_notional.is_finite() || max_notional <= 0.0
    {
        return 0.0;
    }

    let target_notional = if quantity_is_usd {
        quantity
    } else if let Some(reference_price) =
        reference_price.filter(|price| price.is_finite() && *price > 0.0)
    {
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
    if !percentage.is_finite() || !max_notional.is_finite() || max_notional <= 0.0 {
        return "0".to_string();
    }

    let target_notional = max_notional * (percentage.clamp(0.0, 100.0) as f64 / 100.0);
    if quantity_is_usd {
        return format_decimal_with_commas(target_notional, 2);
    }

    if let Some(reference_price) = reference_price.filter(|price| price.is_finite() && *price > 0.0)
    {
        let target_coin = target_notional / reference_price;
        format_decimal_with_commas(target_coin, decimals)
    } else {
        "0".to_string()
    }
}
