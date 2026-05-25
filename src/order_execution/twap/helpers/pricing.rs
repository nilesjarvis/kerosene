use crate::helpers::positive_finite_value;
use crate::signing::round_price;

// ---------------------------------------------------------------------------
// TWAP Price Helpers
// ---------------------------------------------------------------------------

pub(in crate::order_execution::twap) fn twap_ioc_limit_price(
    raw_price: f64,
    is_buy: bool,
    sz_decimals: u32,
    is_spot: bool,
    min_price: f64,
    max_price: f64,
) -> Option<f64> {
    let raw_price = positive_finite_value(raw_price)?;
    let min_price = positive_finite_value(min_price)?;
    let max_price = positive_finite_value(max_price)?;
    if max_price < min_price || raw_price < min_price || raw_price > max_price {
        return None;
    }

    let rounded = round_price(raw_price, sz_decimals, is_spot);
    let rounded = positive_finite_value(rounded)?;

    let price = if is_buy {
        if rounded < raw_price || rounded > max_price {
            raw_price
        } else {
            rounded
        }
    } else if rounded > raw_price || rounded < min_price {
        raw_price
    } else {
        rounded
    };

    positive_finite_value(price)
        .filter(|price| *price >= min_price)
        .filter(|price| *price <= max_price)
}
