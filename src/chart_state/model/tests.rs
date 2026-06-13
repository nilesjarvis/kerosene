use super::*;

mod asset_ctx;
mod order_lines;
mod price_flash;
mod quick_order;
mod surface;

fn instance() -> ChartInstance {
    ChartInstance::new(1, "BTC".to_string(), Timeframe::H1)
}

fn moving_order_overlay() -> crate::chart::OrderOverlay {
    crate::chart::OrderOverlay {
        coin: "BTC".to_string(),
        limit_px: 100.0,
        sz: 1.0,
        is_buy: true,
        oid: 42,
        is_moving: true,
        pending_state: None,
    }
}

fn quick_order_form(
    is_limit: bool,
    quantity: &str,
    quantity_is_usd: bool,
    percentage: f32,
) -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: quantity.to_string(),
        quantity_is_usd,
        percentage,
        quantity_provenance: None,
        is_limit,
        click_x: 10.0,
        click_y: 20.0,
        chart_w: 300.0,
        chart_h: 200.0,
    }
}
