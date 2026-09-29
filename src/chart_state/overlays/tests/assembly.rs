use super::*;
use crate::account::UserFill;
use crate::signing::{ChaseLifecycle, ChaseOrder};

use std::time::Instant;

fn chase(id: u64, oid: Option<u64>, price: f64) -> ChaseOrder {
    let now = Instant::now();
    ChaseOrder {
        id,
        coin: "BTC".to_string(),
        account_address: TEST_ACCOUNT.to_string(),
        agent_key: "test-agent-key".to_string().into(),
        is_buy: true,
        target_size: 2.0,
        filled_size: 0.0,
        remaining_size: 2.0,
        known_oids: oid.into_iter().collect(),
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 3,
        is_spot: false,
        reduce_only: false,
        current_oid: oid,
        current_price: price,
        current_price_wire: price.to_string(),
        initial_price: price,
        started_at: now,
        started_at_ms: 1_000,
        fill_cutoff_ms_by_oid: Vec::new(),
        reprice_count: 0,
        lifecycle: ChaseLifecycle::Resting,
        last_reprice_at: None,
        desired_price: None,
        stop_reason: None,
        cancel_retries: 0,
    }
}

#[test]
fn chart_orders_preserve_chase_precedence_pending_decorations_and_filtering() {
    let mut terminal = terminal_with_btc_chart();
    let modify_order = open_order(42, "A", "100", "1");
    let cancel_order = open_order(43, "A", "95", "0");
    set_open_orders(
        &mut terminal,
        vec![
            modify_order.clone(),
            cancel_order.clone(),
            open_order(44, "B", "NaN", "-1"),
        ],
    );
    for chase in [
        chase(1, Some(42), 105.0),
        chase(2, Some(42), 106.0),
        chase(3, Some(50), 107.0),
        chase(4, Some(51), f64::NAN),
        ChaseOrder {
            account_address: TEST_ACCOUNT.to_ascii_uppercase(),
            ..chase(5, Some(52), 108.0)
        },
        chase(6, None, 109.0),
    ] {
        terminal.chase_orders.insert(chase.id, chase);
    }
    terminal.sync_chart_orders_for(1);
    let orders = chart_orders(&terminal);
    assert_eq!(
        orders.iter().map(|order| order.oid).collect::<Vec<_>>(),
        [42, 43, 44, 50]
    );
    assert_eq!(orders[0].limit_px, 106.0);
    assert_eq!(orders[0].sz, 2.0);
    assert!(orders[0].is_buy);

    terminal.add_pending_order_modification_indicator(
        TEST_ACCOUNT.to_string(),
        &modify_order,
        "110".to_string(),
    );
    terminal.add_pending_order_cancellation_indicator(TEST_ACCOUNT.to_string(), &cancel_order);
    let placing_id = terminal
        .add_pending_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "114".to_string(),
        )
        .expect("valid placement");
    terminal.add_pending_market_order_placement_indicator(
        TEST_ACCOUNT.to_string(),
        "BTC".to_string(),
        true,
        "0.25".to_string(),
        "120".to_string(),
    );
    terminal.add_pending_order_placement_indicator(
        "another-account".to_string(),
        "BTC".to_string(),
        true,
        "1".to_string(),
        "115".to_string(),
    );
    terminal.sync_chart_orders_for(1);

    let orders = chart_orders(&terminal);
    assert_eq!(
        orders.iter().map(|order| order.oid).collect::<Vec<_>>(),
        [42, 43, 44, 50, placing_id]
    );
    assert_eq!(
        (orders[0].limit_px, orders[0].sz, orders[0].is_buy),
        (110.0, 1.0, false)
    );
    assert_eq!(
        orders[0].pending_state,
        Some(OrderOverlayPendingState::Modifying)
    );
    assert_eq!(orders[1].sz, 0.0);
    assert_eq!(
        orders[1].pending_state,
        Some(OrderOverlayPendingState::Cancelling)
    );
    // Confirmed orders keep their existing parsing policy, unlike Chase rows.
    assert!(orders[2].limit_px.is_nan());
    assert_eq!(orders[2].sz, -1.0);
    assert_eq!(orders[3].limit_px, 107.0);
    assert_eq!(
        orders[4].pending_state,
        Some(OrderOverlayPendingState::Placing)
    );
    assert!(
        terminal
            .charts
            .get(&1)
            .expect("chart")
            .chart
            .hud_order_animation_active()
    );

    terminal.muted_tickers.insert("BTC".to_string());
    terminal.sync_chart_orders_for(1);
    assert!(chart_orders(&terminal).is_empty());
    assert!(
        !terminal
            .charts
            .get(&1)
            .expect("chart")
            .chart
            .hud_order_animation_active()
    );
}

fn fill(coin: &str, time: u64, price: &str, side: &str) -> UserFill {
    UserFill {
        coin: coin.to_string(),
        px: price.to_string(),
        sz: "0.25".to_string(),
        side: side.to_string(),
        time,
        hash: None,
        tid: None,
        oid: None,
        dir: "Open Long".to_string(),
        closed_pnl: "0".to_string(),
        fee: "0".to_string(),
        fee_token: None,
        start_position: None,
    }
}

#[test]
fn chart_positions_and_trades_preserve_instance_account_and_symbol_boundaries() {
    let mut terminal = terminal_with_btc_chart();
    for (id, symbol) in [(2, "ETH"), (3, "BTC")] {
        terminal.charts.insert(
            id,
            ChartInstance::new(id, symbol.to_string(), Timeframe::H1),
        );
    }
    let mut data = account_data_with_leverage("BTC", 1);
    let mut eth = data.clearinghouse.asset_positions[0].clone();
    eth.position.coin = "ETH".to_string();
    eth.position.szi = "-2".to_string();
    eth.position.entry_px = "50".to_string();
    eth.position.liquidation_px = Some("80".to_string());
    data.clearinghouse.asset_positions.push(eth);
    data.fills = vec![
        fill("BTC", 2, "100", "B"),
        fill("ETH", 1, "50", "A"),
        fill("BTC", 1, "101", "A"),
        fill("BTC", 1, "102", "B"),
        fill("BTC", 3, "NaN", "B"),
        fill("btc", 4, "103", "B"),
    ];
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);
    terminal.connected_address = Some(format!(" {} ", TEST_ACCOUNT.to_ascii_uppercase()));
    terminal.sync_all_chart_overlays();

    for id in [1, 3] {
        let chart = &terminal.charts.get(&id).expect("BTC chart").chart;
        let position = chart.active_position.as_ref().expect("BTC position");
        assert_eq!(
            (position.entry_px, position.szi, position.liquidation_px),
            (100.0, 1.0, None)
        );
        assert_eq!(
            chart
                .trade_markers
                .iter()
                .map(|marker| (marker.time_ms, marker.price, marker.is_buy))
                .collect::<Vec<_>>(),
            [(1, 101.0, false), (1, 102.0, true), (2, 100.0, true)]
        );
    }
    let eth = &terminal.charts.get(&2).expect("ETH chart").chart;
    let position = eth.active_position.as_ref().expect("ETH position");
    assert_eq!(
        (position.entry_px, position.szi, position.liquidation_px),
        (50.0, -2.0, Some(80.0))
    );
    assert_eq!(eth.trade_markers.len(), 1);
    assert_eq!(eth.trade_markers[0].price, 50.0);

    terminal.muted_tickers.insert("BTC".to_string());
    terminal.sync_all_chart_overlays();
    for id in [1, 3] {
        let chart = &terminal.charts.get(&id).expect("BTC chart").chart;
        assert!(chart.active_position.is_none());
        assert!(chart.trade_markers.is_empty());
    }
    assert!(
        terminal
            .charts
            .get(&2)
            .expect("ETH chart")
            .chart
            .active_position
            .is_some()
    );

    terminal.connected_address = Some("another-account".to_string());
    terminal.sync_all_chart_overlays();
    for instance in terminal.charts.values() {
        assert!(instance.chart.active_position.is_none());
        assert!(instance.chart.trade_markers.is_empty());
    }
}
