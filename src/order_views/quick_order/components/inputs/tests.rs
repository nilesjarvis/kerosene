use super::quick_order_fee_quantity;
use crate::order_execution::QuickOrderForm;

fn quick_order_form(quantity: &str, quantity_is_usd: bool) -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: quantity.to_string(),
        quantity_is_usd,
        percentage: 0.0,
        is_limit: true,
        click_x: 0.0,
        click_y: 0.0,
        chart_w: 0.0,
        chart_h: 0.0,
    }
}

#[test]
fn quick_order_usd_fee_quantity_converts_notional_to_base_size() {
    let form = quick_order_form("250", true);
    assert_eq!(quick_order_fee_quantity(&form, 100.0, 5), Some(2.5));
}

#[test]
fn quick_order_coin_fee_quantity_uses_asset_precision() {
    let form = quick_order_form("1.239", false);
    assert_eq!(quick_order_fee_quantity(&form, 100.0, 2), Some(1.23));
}
