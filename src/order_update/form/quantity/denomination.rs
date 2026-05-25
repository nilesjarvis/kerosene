// ---------------------------------------------------------------------------
// Denomination Quantity Math
// ---------------------------------------------------------------------------

use crate::helpers::{format_decimal_with_commas, positive_finite_value};

pub(in crate::order_update::form) fn toggled_order_quantity_text(
    quantity: f64,
    now_quantity_is_usd: bool,
    reference_price: f64,
    decimals: usize,
) -> Option<String> {
    let quantity = positive_finite_value(quantity)?;
    let reference_price = positive_finite_value(reference_price)?;

    let new_quantity = if now_quantity_is_usd {
        quantity * reference_price
    } else {
        quantity / reference_price
    };

    let new_quantity = positive_finite_value(new_quantity)?;

    if now_quantity_is_usd {
        Some(format_decimal_with_commas(new_quantity, 2))
    } else {
        Some(format_decimal_with_commas(new_quantity, decimals))
    }
}
