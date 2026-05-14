// ---------------------------------------------------------------------------
// Denomination Quantity Math
// ---------------------------------------------------------------------------

use crate::helpers::format_decimal_with_commas;

pub(in crate::order_update::form) fn toggled_order_quantity_text(
    quantity: f64,
    now_quantity_is_usd: bool,
    reference_price: f64,
    decimals: usize,
) -> Option<String> {
    if !quantity.is_finite()
        || quantity <= 0.0
        || !reference_price.is_finite()
        || reference_price <= 0.0
    {
        return None;
    }

    let new_quantity = if now_quantity_is_usd {
        quantity * reference_price
    } else {
        quantity / reference_price
    };

    if !new_quantity.is_finite() {
        return None;
    }

    if now_quantity_is_usd {
        Some(format_decimal_with_commas(new_quantity, 2))
    } else {
        Some(format_decimal_with_commas(new_quantity, decimals))
    }
}
