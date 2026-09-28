use super::*;
use crate::account::{
    AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
    Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType};
use crate::chart::OrderOverlayPendingState;
use crate::chart_state::ChartInstance;
use crate::timeframe::Timeframe;

mod assembly;

fn symbol(key: &str) -> ExchangeSymbol {
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
        growth_mode: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn account_data_with_leverage(coin: &str, leverage: u32) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "1000".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "1000".to_string(),
            asset_positions: vec![AssetPosition {
                position: Position {
                    coin: coin.to_string(),
                    szi: "1".to_string(),
                    entry_px: "100".to_string(),
                    position_value: "100".to_string(),
                    unrealized_pnl: "0".to_string(),
                    liquidation_px: None,
                    leverage: PositionLeverage {
                        leverage_type: "cross".to_string(),
                        value: leverage,
                    },
                    margin_used: "0".to_string(),
                    cum_funding: None,
                },
                liquidation_px: None,
            }],
        },
        clearinghouses_by_dex: std::collections::HashMap::new(),
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms: TradingTerminal::now_ms(),
    }
}

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

fn open_order(oid: u64, side: &str, limit_px: &str, sz: &str) -> crate::account::OpenOrder {
    crate::account::OpenOrder {
        coin: "BTC".to_string(),
        side: side.to_string(),
        limit_px: limit_px.to_string(),
        sz: sz.to_string(),
        oid,
        timestamp: 1,
        reduce_only: Some(false),
        is_trigger: None,
        order_type: None,
        tif: None,
        trigger_px: None,
    }
}

fn terminal_with_btc_chart() -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.charts.clear();
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
    terminal
}

fn set_open_orders(terminal: &mut TradingTerminal, orders: Vec<crate::account::OpenOrder>) {
    let mut data = account_data_with_leverage("BTC", 1);
    data.open_orders = orders;
    let address = terminal
        .connected_address
        .clone()
        .expect("test connected account");
    terminal.set_account_data_for_address_for_test(address, data);
}

fn chart_orders(terminal: &TradingTerminal) -> &[crate::chart::OrderOverlay] {
    &terminal.charts.get(&1).unwrap().chart.active_orders
}

#[test]
fn placing_indicator_suppressed_once_matching_confirmed_order_arrives() {
    let mut terminal = terminal_with_btc_chart();
    terminal.add_pending_order_placement_indicator(
        TEST_ACCOUNT.to_string(),
        "BTC".to_string(),
        true,
        "1".to_string(),
        "100".to_string(),
    );
    assert_eq!(chart_orders(&terminal).len(), 1);

    // The websocket can deliver the confirmed order before the place ack;
    // differing wire formats for the same values must still match.
    set_open_orders(&mut terminal, vec![open_order(42, "B", "100.0", "1.0")]);
    terminal.sync_all_chart_orders();

    let orders = chart_orders(&terminal);
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].oid, 42);
    assert!(orders[0].pending_state.is_none());
}

#[test]
fn placing_indicator_persists_while_no_confirmed_order_matches() {
    let mut terminal = terminal_with_btc_chart();
    terminal.add_pending_order_placement_indicator(
        TEST_ACCOUNT.to_string(),
        "BTC".to_string(),
        true,
        "1".to_string(),
        "100".to_string(),
    );

    set_open_orders(&mut terminal, vec![open_order(42, "B", "101", "1")]);
    terminal.sync_all_chart_orders();

    let orders = chart_orders(&terminal);
    assert_eq!(orders.len(), 2);
    assert_eq!(
        orders
            .iter()
            .filter(|order| order.pending_state == Some(OrderOverlayPendingState::Placing))
            .count(),
        1
    );
}

#[test]
fn cancelling_indicator_decorates_live_order_line() {
    let mut terminal = terminal_with_btc_chart();
    let order = open_order(42, "B", "100", "1");
    set_open_orders(&mut terminal, vec![order.clone()]);
    terminal.add_pending_order_cancellation_indicator(TEST_ACCOUNT.to_string(), &order);

    let orders = chart_orders(&terminal);
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].oid, 42);
    assert_eq!(
        orders[0].pending_state,
        Some(OrderOverlayPendingState::Cancelling)
    );
}

#[test]
fn cancelling_indicator_decorates_zero_size_trigger_order_line() {
    let mut terminal = terminal_with_btc_chart();
    // Position-tied TP/SL trigger orders carry sz "0.0"; the decoration
    // needs no price/size of its own.
    let order = open_order(42, "A", "100", "0.0");
    set_open_orders(&mut terminal, vec![order.clone()]);
    let pending_id =
        terminal.add_pending_order_cancellation_indicator(TEST_ACCOUNT.to_string(), &order);
    assert!(pending_id.is_some());

    let orders = chart_orders(&terminal);
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].oid, 42);
    assert_eq!(
        orders[0].pending_state,
        Some(OrderOverlayPendingState::Cancelling)
    );
}

#[test]
fn cancelling_indicator_does_not_resurrect_missing_order() {
    let mut terminal = terminal_with_btc_chart();
    let order = open_order(42, "B", "100", "1");
    set_open_orders(&mut terminal, vec![order.clone()]);
    terminal.add_pending_order_cancellation_indicator(TEST_ACCOUNT.to_string(), &order);

    // The websocket removes the cancelled order before the cancel ack
    // arrives; the indicator must not re-draw a line for it.
    set_open_orders(&mut terminal, Vec::new());
    terminal.sync_all_chart_orders();

    assert!(chart_orders(&terminal).is_empty());
    assert!(!terminal.pending_order_indicators.is_empty());
}

#[test]
fn market_reference_prices_sync_from_live_mids() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.charts.clear();
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
    terminal.all_mids.insert("BTC".to_string(), 50_000.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    terminal.sync_chart_market_reference_prices();

    assert_eq!(
        terminal
            .charts
            .get(&1)
            .unwrap()
            .chart
            .market_reference_price,
        Some(50_000.0)
    );
}

#[test]
fn hud_max_notional_syncs_from_visible_margin_and_actual_leverage() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.charts.clear();
    terminal.exchange_symbols = vec![symbol("BTC")];
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal
        .set_account_data_for_address_for_test(TEST_ACCOUNT, account_data_with_leverage("BTC", 10));
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));

    terminal.sync_chart_market_reference_prices();

    assert_eq!(
        terminal.charts.get(&1).unwrap().chart.hud_max_notional,
        Some(10_000.0)
    );
}
