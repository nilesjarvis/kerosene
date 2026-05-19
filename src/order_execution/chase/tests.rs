use super::*;
use crate::api::{ExchangeSymbol, MarketType};
use crate::chart::OrderOverlay;
use crate::chart_state::ChartInstance;
use crate::signing::OrderKind;
use crate::timeframe::Timeframe;

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 50,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn chase_ready_terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.wallet_key_input = "agent-key".to_string().into();
    terminal.connected_address = Some("0xabc".to_string());
    terminal.order_kind = OrderKind::Chase;
    terminal.order_quantity = "2.5".to_string();
    terminal.order_quantity_is_usd = false;
    terminal.pending_order_action = None;
    terminal
}

#[test]
fn start_chase_keeps_base_size_when_quantity_is_not_usd() {
    let mut terminal = chase_ready_terminal();

    let _task = terminal.start_chase(true);

    let chase = terminal
        .selected_chase()
        .expect("chase order should be inserted");
    assert_eq!(chase.target_size, 2.5);
    assert_eq!(chase.remaining_size, 2.5);
}

#[test]
fn start_chase_quantizes_base_size_to_asset_precision() {
    let mut terminal = chase_ready_terminal();
    terminal.order_quantity = "1.239".to_string();
    if let Some(symbol) = terminal.exchange_symbols.first_mut() {
        symbol.sz_decimals = 2;
    }

    let _task = terminal.start_chase(true);

    let chase = terminal
        .selected_chase()
        .expect("chase order should be inserted");
    assert_eq!(chase.target_size, 1.23);
    assert_eq!(chase.remaining_size, 1.23);
}

#[test]
fn start_chase_converts_usd_notional_to_base_size_using_fresh_mid() {
    let mut terminal = chase_ready_terminal();
    terminal.order_quantity = "1000".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.all_mids.insert("BTC".to_string(), 50_000.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let _task = terminal.start_chase(true);

    let chase = terminal
        .selected_chase()
        .expect("chase order should be inserted");
    assert!((chase.target_size - 0.02).abs() < f64::EPSILON);
    assert!((chase.remaining_size - 0.02).abs() < f64::EPSILON);
}

#[test]
fn start_chase_converts_usd_notional_using_mid_candidates() {
    let mut terminal = chase_ready_terminal();
    terminal.active_symbol = "UBTC".to_string();
    terminal.active_symbol_display = "UBTC".to_string();
    terminal.exchange_symbols = vec![symbol("UBTC", MarketType::Perp)];
    terminal.order_quantity = "1000".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.all_mids.insert("BTC".to_string(), 50_000.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let _task = terminal.start_chase(true);

    let chase = terminal
        .selected_chase()
        .expect("chase order should be inserted");
    assert!((chase.target_size - 0.02).abs() < f64::EPSILON);
    assert!((chase.remaining_size - 0.02).abs() < f64::EPSILON);
}

#[test]
fn start_chase_rejects_usd_notional_without_fresh_mid() {
    let mut terminal = chase_ready_terminal();
    terminal.order_quantity = "1000".to_string();
    terminal.order_quantity_is_usd = true;

    let _task = terminal.start_chase(true);

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| *is_error && message.contains("no fresh mid price")),
        "status should explain the missing fresh mid: {:?}",
        terminal.order_status
    );
}

#[test]
fn start_chase_rejects_usd_notional_with_stale_mid() {
    let mut terminal = chase_ready_terminal();
    terminal.order_quantity = "1000".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.all_mids.insert("BTC".to_string(), 50_000.0);
    terminal.all_mids_updated_at_ms.insert("BTC".to_string(), 0);

    let _task = terminal.start_chase(true);

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
}

#[test]
fn removing_chase_order_resyncs_chart_order_overlays() {
    let mut terminal = chase_ready_terminal();
    terminal.charts.clear();
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
    terminal
        .charts
        .get_mut(&1)
        .expect("chart")
        .chart
        .active_orders
        .push(OrderOverlay {
            coin: "BTC".to_string(),
            limit_px: 100.0,
            sz: 1.0,
            is_buy: true,
            oid: 42,
            is_moving: false,
        });

    let _task =
        terminal.handle_chase_resting_order("BTC".to_string(), 42, true, 1.0, 100.0, Some(false));
    let chase_id = terminal
        .selected_chase_id()
        .expect("resting chase should be selected");

    terminal.remove_chase_order(chase_id);

    assert!(
        terminal
            .charts
            .get(&1)
            .expect("chart")
            .chart
            .active_orders
            .is_empty()
    );
}
