// ---------------------------------------------------------------------------
// Order Size Helpers
// ---------------------------------------------------------------------------

use crate::helpers::positive_finite_value;

pub(crate) fn order_size_from_quantity_input(
    raw_quantity: f64,
    price: f64,
    quantity_is_usd: bool,
    sz_decimals: u32,
) -> Option<f64> {
    let raw_quantity = positive_finite_value(raw_quantity)?;

    let size = if quantity_is_usd {
        let price = positive_finite_value(price)?;
        raw_quantity / price
    } else {
        raw_quantity
    };

    quantize_order_size(size, sz_decimals)
}

pub(crate) fn quantize_order_size(size: f64, sz_decimals: u32) -> Option<f64> {
    let size = positive_finite_value(size)?;

    let decimals = sz_decimals.min(8);
    let factor = 10f64.powi(decimals as i32);
    let quantized = ((size * factor) + 1e-9).floor() / factor;
    positive_finite_value(quantized)
}

#[cfg(test)]
mod tests;
