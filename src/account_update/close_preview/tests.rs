use super::*;
use crate::account::{
    AccountData, ClearinghouseState, MarginSummary, SpotClearinghouseState, UserFeeRates,
};

fn terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot_from_config(Default::default()).0;
    let address = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(address.into());
    let mut data = AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "1000".into(),
                total_ntl_pos: "200".into(),
                total_margin_used: "20".into(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "980".into(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: Default::default(),
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates {
            user_cross_rate: "0.001".into(),
            ..Default::default()
        },
        completeness: Default::default(),
        fetched_at_ms: terminal.status_bar_now_ms,
    };
    data.clearinghouse.asset_positions.push(
        serde_json::from_value(serde_json::json!({"position": {
            "coin": "BTC", "szi": "2", "entryPx": "100", "positionValue": "200",
            "unrealizedPnl": "0", "leverage": {"type": "cross", "value": 1}
        }}))
        .expect("synthetic position"),
    );
    terminal.set_account_data_for_address_for_test(address, data);
    terminal
}

#[test]
fn hover_exit_reentry_and_late_events_are_scoped() {
    let mut terminal = terminal();
    let source = terminal.market_data_source_context();
    let _ = terminal.hover_position_close_preview("BTC".into(), true);
    let generation = terminal.position_close_preview.generation;
    let _ = terminal.apply_position_close_book(
        generation,
        "BTC".into(),
        source,
        Some(OrderBook::empty()),
    );
    assert!(terminal.position_close_preview.book.is_some());
    let _ = terminal.hover_position_close_preview("ETH".into(), false);
    assert!(terminal.position_close_preview.book.is_some());
    let _ = terminal.hover_position_close_preview("BTC".into(), false);
    assert!(terminal.position_close_preview.coin.is_none());
    let _ = terminal.hover_position_close_preview("BTC".into(), true);
    let _ = terminal.apply_position_close_book(
        generation,
        "BTC".into(),
        source,
        Some(OrderBook::empty()),
    );
    assert!(terminal.position_close_preview.book.is_none());
    let _ = terminal.apply_position_close_fee(generation, "BTC".into(), Ok(99.0));
    assert!(!matches!(
        terminal.position_close_preview.fee_multiplier,
        Some(Ok(99.0))
    ));
}

#[test]
fn privacy_account_switch_source_changes_and_lag_invalidate_preview() {
    let mut terminal = terminal();
    let source = terminal.market_data_source_context();
    let _ = terminal.hover_position_close_preview("BTC".into(), true);
    let generation = terminal.position_close_preview.generation;
    let _ = terminal.apply_position_close_book(
        generation,
        "BTC".into(),
        source,
        Some(OrderBook::empty()),
    );
    let _ = terminal.apply_position_close_book(generation, "BTC".into(), source, None);
    assert!(terminal.position_close_preview.book.is_none());
    terminal.read_data_provider_generation += 1;
    let _ = terminal.apply_position_close_book(
        generation,
        "BTC".into(),
        source,
        Some(OrderBook::empty()),
    );
    assert!(terminal.position_close_preview.book.is_none());
    terminal.hide_pnl = true;
    assert!(!terminal.position_close_preview_is_visible("BTC"));
    let _ = terminal.hover_position_close_preview("BTC".into(), true);
    assert!(terminal.position_close_preview.coin.is_none());
    terminal.hide_pnl = false;
    let _ = terminal.hover_position_close_preview("BTC".into(), true);
    terminal.clear_account_scoped_chart_state();
    assert!(terminal.position_close_preview.coin.is_none());
}
