use super::{asset_position, position_size_for_symbol, terminal_with_position};
use crate::account::{ClearinghouseState, MarginSummary};

#[test]
fn reduce_only_slider_sizes_coin_quantity_from_position() {
    let mut terminal = terminal_with_position("BTC", "2.5");
    terminal.order_reduce_only = true;
    terminal.order_quantity_is_usd = false;

    terminal.handle_order_percentage_changed(50.0);

    assert_eq!(terminal.order_quantity, "1.25000");
}

#[test]
fn reduce_only_slider_sizes_usd_quantity_from_position_notional() {
    let mut terminal = terminal_with_position("BTC", "2");
    terminal.order_reduce_only = true;
    terminal.order_quantity_is_usd = true;
    terminal.order_kind = crate::signing::OrderKind::Limit;
    terminal.order_price = "100".to_string();

    terminal.handle_order_percentage_changed(25.0);

    assert_eq!(terminal.order_quantity, "50.00");
}

#[test]
fn reduce_only_manual_quantity_updates_percentage_from_position() {
    let mut terminal = terminal_with_position("BTC", "-2");
    terminal.order_reduce_only = true;
    terminal.order_quantity_is_usd = false;

    terminal.handle_order_quantity_changed("0.5".to_string());

    assert_eq!(terminal.order_percentage, 25.0);
}

#[test]
fn reduce_only_toggle_resizes_existing_slider_percentage_to_position() {
    let mut terminal = terminal_with_position("BTC", "2");
    terminal.order_reduce_only = false;
    terminal.order_quantity_is_usd = false;
    terminal.order_percentage = 50.0;

    terminal.handle_toggle_reduce_only();

    assert!(terminal.order_reduce_only);
    assert_eq!(terminal.order_quantity, "1.00000");
}

#[test]
fn reduce_only_slider_without_active_position_does_not_use_opening_margin() {
    let mut terminal = terminal_with_position("ETH", "2");
    terminal.order_reduce_only = true;
    terminal.order_quantity_is_usd = false;

    terminal.handle_order_percentage_changed(50.0);

    assert_eq!(terminal.order_quantity, "0");
}

#[test]
fn reduce_only_position_lookup_prefers_exact_active_symbol() {
    let clearinghouse = ClearinghouseState {
        margin_summary: MarginSummary {
            account_value: "0".to_string(),
            total_ntl_pos: "0".to_string(),
            total_margin_used: "0".to_string(),
        },
        cross_margin_summary: None,
        cross_maintenance_margin_used: None,
        withdrawable: "0".to_string(),
        asset_positions: vec![asset_position("BTC", "1"), asset_position("xyz:BTC", "3")],
    };

    assert_eq!(
        position_size_for_symbol(&clearinghouse, "xyz:BTC"),
        Some(3.0)
    );
    assert_eq!(position_size_for_symbol(&clearinghouse, "BTC"), Some(1.0));
}
