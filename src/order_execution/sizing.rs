// ---------------------------------------------------------------------------
// Order Size Helpers
// ---------------------------------------------------------------------------

pub(crate) fn order_size_from_quantity_input(
    raw_quantity: f64,
    price: f64,
    quantity_is_usd: bool,
    sz_decimals: u32,
) -> Option<f64> {
    if !raw_quantity.is_finite() || raw_quantity <= 0.0 {
        return None;
    }

    let size = if quantity_is_usd {
        if !price.is_finite() || price <= 0.0 {
            return None;
        }
        raw_quantity / price
    } else {
        raw_quantity
    };

    quantize_order_size(size, sz_decimals)
}

pub(crate) fn quantize_order_size(size: f64, sz_decimals: u32) -> Option<f64> {
    if !size.is_finite() || size <= 0.0 {
        return None;
    }

    let decimals = sz_decimals.min(8);
    let factor = 10f64.powi(decimals as i32);
    let quantized = ((size * factor) + 1e-9).floor() / factor;
    (quantized.is_finite() && quantized > 0.0).then_some(quantized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_size_keeps_coin_amount_at_valid_precision() {
        assert_eq!(
            order_size_from_quantity_input(2.5, 100.0, false, 5),
            Some(2.5)
        );
    }

    #[test]
    fn order_size_quantizes_coin_amount_to_asset_precision() {
        assert_eq!(
            order_size_from_quantity_input(1.23, 100.0, false, 2),
            Some(1.23)
        );
        assert_eq!(
            order_size_from_quantity_input(1.239, 100.0, false, 2),
            Some(1.23)
        );
    }

    #[test]
    fn order_size_converts_usd_amount_by_price() {
        assert_eq!(
            order_size_from_quantity_input(250.0, 100.0, true, 5),
            Some(2.5)
        );
    }

    #[test]
    fn order_size_quantizes_usd_conversion_to_asset_precision() {
        assert_eq!(
            order_size_from_quantity_input(10.0, 30_000.0, true, 5),
            Some(0.00033)
        );
    }

    #[test]
    fn order_size_rejects_size_below_asset_precision() {
        assert_eq!(
            order_size_from_quantity_input(10.0, 30_000.0, true, 2),
            None
        );
        assert_eq!(order_size_from_quantity_input(0.009, 100.0, false, 2), None);
    }

    #[test]
    fn order_size_rejects_invalid_raw_quantity_or_conversion_price() {
        assert_eq!(order_size_from_quantity_input(0.0, 100.0, false, 5), None);
        assert_eq!(
            order_size_from_quantity_input(f64::NAN, 100.0, false, 5),
            None
        );
        assert_eq!(order_size_from_quantity_input(250.0, 0.0, true, 5), None);
        assert_eq!(
            order_size_from_quantity_input(250.0, f64::INFINITY, true, 5),
            None
        );
    }
}
