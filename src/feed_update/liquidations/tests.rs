use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::{HydromancerWsMessage, LiquidationEvent};
use std::collections::{BTreeMap, VecDeque};

fn liquidation_event(time_ms: u64, is_buy: bool, size: f64, price: f64) -> LiquidationEvent {
    LiquidationEvent {
        coin: "HYPE".to_string(),
        price,
        size,
        is_buy,
        time_ms,
        method: "market".to_string(),
        liquidated_user: "synthetic-user".to_string(),
        tx_index: 1,
    }
}

#[test]
fn live_liquidations_keep_bucket_sides_retention_and_bounded_history() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.liquidation_alerts_enabled = false;
    terminal.liquidations = VecDeque::from(vec![liquidation_event(0, false, 1.0, 1.0); 10_000]);
    terminal.liquidation_summary_buckets.insert(0, (1.0, 1.0));
    terminal.liquidation_chart_buckets.insert(0, (1.0, 1.0));
    // Future bucket boundaries avoid crossing a retention/minute boundary during the test.
    let base = (TradingTerminal::now_ms() / 60_000 + 1) * 60_000;
    for event in [
        liquidation_event(base, false, 2.0, 5.0),
        liquidation_event(base + 1_000, true, 4.0, 2.0),
        liquidation_event(base + 1_000, false, 3.0, 4.0),
    ] {
        let message = scoped_liquidation_message(&terminal, HydromancerWsMessage::Event(event));
        let _ = terminal.update_liquidation_feed(message);
    }

    assert_eq!(terminal.liquidations.len(), 10_000);
    let latest = terminal
        .liquidations
        .front()
        .expect("latest liquidation should be retained");
    assert_eq!(latest.time_ms, base + 1_000);
    assert_eq!(latest.size, 3.0);
    assert_eq!(
        terminal.liquidation_summary_buckets,
        BTreeMap::from([(base / 60_000, (22.0, 8.0))])
    );
    assert_eq!(
        terminal.liquidation_chart_buckets,
        BTreeMap::from([(base / 1_000, (10.0, 0.0)), (base / 1_000 + 1, (12.0, 8.0)),])
    );
}

#[test]
fn recomputed_liquidation_buckets_include_all_history_without_pruning() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.liquidations = VecDeque::from([
        liquidation_event(0, false, 1.0, 2.0),
        liquidation_event(59_999, true, 3.0, 2.0),
        liquidation_event(60_000, false, 2.0, 5.0),
        liquidation_event(60_999, true, 4.0, 2.0),
        liquidation_event(61_000, false, 5.0, 2.0),
    ]);
    terminal.liquidation_summary_buckets.insert(999, (1.0, 1.0));
    terminal.liquidation_chart_buckets.insert(999, (1.0, 1.0));

    terminal.recompute_liquidation_buckets();

    assert_eq!(
        terminal.liquidation_summary_buckets,
        BTreeMap::from([(0, (2.0, 6.0)), (1, (20.0, 8.0))])
    );
    assert_eq!(
        terminal.liquidation_chart_buckets,
        BTreeMap::from([
            (0, (2.0, 0.0)),
            (59, (0.0, 6.0)),
            (60, (10.0, 8.0)),
            (61, (10.0, 0.0)),
        ])
    );
}

#[test]
fn hidden_liquidation_refreshes_status_without_rows_or_buckets() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.muted_tickers.insert("HYPE".to_string());
    terminal.liquidations_status = "Current".to_string();
    terminal.liquidations_last_rx_ms = Some(123);
    let before = TradingTerminal::now_ms();
    let message = scoped_liquidation_message(
        &terminal,
        HydromancerWsMessage::Event(liquidation_event(before, false, 1.0, 1.0)),
    );

    let _ = terminal.update_liquidation_feed(message);

    assert_eq!(terminal.liquidations_status, "Connected");
    assert!(
        terminal
            .liquidations_last_rx_ms
            .is_some_and(|last_rx| last_rx >= before)
    );
    assert!(terminal.liquidations.is_empty());
    assert!(terminal.liquidation_summary_buckets.is_empty());
    assert!(terminal.liquidation_chart_buckets.is_empty());
    assert!(terminal.toasts.is_empty());
}

fn scoped_liquidation_message(
    terminal: &TradingTerminal,
    message: HydromancerWsMessage,
) -> Message {
    Message::WsHydromancerLiquidation {
        hydromancer_key_generation: terminal.hydromancer_key_generation,
        reconnect_nonce: terminal.liquidations_reconnect_nonce,
        message,
    }
}

fn scoped_liquidation_message_with(
    hydromancer_key_generation: u64,
    reconnect_nonce: u64,
    message: HydromancerWsMessage,
) -> Message {
    Message::WsHydromancerLiquidation {
        hydromancer_key_generation,
        reconnect_nonce,
        message,
    }
}

fn outcome_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT95-YES".to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        growth_mode: false,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 95,
            contract: crate::api::OutcomeContract::verified_fixture(),
            venue: None,
            question_id: None,
            question_name: Some("Will BTC close green?".to_string()),
            question_description: None,
            question_class: None,
            question_underlying: None,
            question_expiry: None,
            question_price_thresholds: Vec::new(),
            question_period: None,
            question_named_outcomes: Vec::new(),
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: None,
            bucket_index: None,
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring".to_string(),
            description: "Will BTC close green?".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 950,
        }),
    }
}

#[test]
fn liquidation_alert_toast_resolves_outcome_coin_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(outcome_symbol("#950"));
    terminal.liquidation_alerts_enabled = true;
    terminal.liquidation_alert_threshold = 1.0;
    let liquidation = LiquidationEvent {
        coin: "#950".to_string(),
        price: 0.5,
        size: 100.0,
        is_buy: false,
        time_ms: TradingTerminal::now_ms(),
        method: "market".to_string(),
        liquidated_user: "0x0000000000000000000000000000000000000001".to_string(),
        tx_index: 1,
    };

    let message = scoped_liquidation_message(&terminal, HydromancerWsMessage::Event(liquidation));
    let _ = terminal.update_liquidation_feed(message);

    let toast = terminal.toasts.last().expect("liquidation alert toast");
    assert!(
        toast.message.contains("YES: Will BTC close green?"),
        "{}",
        toast.message
    );
    assert!(!toast.message.contains("#950"), "{}", toast.message);
}

#[test]
fn clear_liquidations_resets_rows_summary_and_chart_buckets() {
    let mut terminal = TradingTerminal::boot().0;
    let liquidation = LiquidationEvent {
        coin: "HYPE".to_string(),
        price: 25.0,
        size: 4.0,
        is_buy: false,
        time_ms: TradingTerminal::now_ms(),
        method: "market".to_string(),
        liquidated_user: "0x0000000000000000000000000000000000000001".to_string(),
        tx_index: 1,
    };

    let message = scoped_liquidation_message(&terminal, HydromancerWsMessage::Event(liquidation));
    let _ = terminal.update_liquidation_feed(message);
    assert!(!terminal.liquidations.is_empty());
    assert!(!terminal.liquidation_summary_buckets.is_empty());
    assert!(!terminal.liquidation_chart_buckets.is_empty());

    let _ = terminal.update_liquidation_feed(Message::ClearLiquidations);

    assert!(terminal.liquidations.is_empty());
    assert!(terminal.liquidation_summary_buckets.is_empty());
    assert!(terminal.liquidation_chart_buckets.is_empty());
}

#[test]
fn lagged_liquidation_stream_marks_stale_and_clears_derived_buckets() {
    let mut terminal = TradingTerminal::boot().0;
    let liquidation = LiquidationEvent {
        coin: "HYPE".to_string(),
        price: 25.0,
        size: 4.0,
        is_buy: false,
        time_ms: TradingTerminal::now_ms(),
        method: "market".to_string(),
        liquidated_user: "0x0000000000000000000000000000000000000001".to_string(),
        tx_index: 1,
    };

    let message = scoped_liquidation_message(&terminal, HydromancerWsMessage::Event(liquidation));
    let _ = terminal.update_liquidation_feed(message);
    assert!(terminal.liquidations_last_rx_ms.is_some());
    assert!(!terminal.liquidations.is_empty());
    assert!(!terminal.liquidation_summary_buckets.is_empty());
    assert!(!terminal.liquidation_chart_buckets.is_empty());

    let message =
        scoped_liquidation_message(&terminal, HydromancerWsMessage::Lagged { skipped: 7 });
    let _ = terminal.update_liquidation_feed(message);

    assert!(terminal.liquidations_last_rx_ms.is_none());
    assert_eq!(
        terminal.liquidations_status,
        "Stream lagged; reconnecting after skipping 7 messages"
    );
    assert!(!terminal.liquidations.is_empty());
    assert!(terminal.liquidation_summary_buckets.is_empty());
    assert!(terminal.liquidation_chart_buckets.is_empty());
}

#[test]
fn liquidation_reconnect_status_redacts_sensitive_hydromancer_error_values() {
    let mut terminal = TradingTerminal::boot().0;
    let message = scoped_liquidation_message(
        &terminal,
        HydromancerWsMessage::Reconnecting {
            error: "failed wss://api.hydromancer.xyz/ws?Token=hydro-secret&sessionId=session-secret&CURSOR=cursor-secret".to_string(),
            retry_delay_secs: 7,
        },
    );

    let _ = terminal.update_liquidation_feed(message);

    assert!(terminal.liquidations_status.contains("<redacted>"));
    for secret in ["hydro-secret", "session-secret", "cursor-secret"] {
        assert!(
            !terminal.liquidations_status.contains(secret),
            "status leaked {secret}: {}",
            terminal.liquidations_status
        );
    }
}

#[test]
fn stale_liquidation_stream_scope_is_ignored() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.hydromancer_key_generation = 2;
    terminal.liquidations_reconnect_nonce = 3;
    terminal.liquidations_status = "Current".to_string();
    let liquidation = LiquidationEvent {
        coin: "HYPE".to_string(),
        price: 25.0,
        size: 4.0,
        is_buy: false,
        time_ms: TradingTerminal::now_ms(),
        method: "market".to_string(),
        liquidated_user: "0x0000000000000000000000000000000000000001".to_string(),
        tx_index: 1,
    };

    let message = scoped_liquidation_message_with(
        1,
        terminal.liquidations_reconnect_nonce,
        HydromancerWsMessage::Event(liquidation.clone()),
    );
    let _ = terminal.update_liquidation_feed(message);
    let message = scoped_liquidation_message_with(
        terminal.hydromancer_key_generation,
        2,
        HydromancerWsMessage::Connected,
    );
    let _ = terminal.update_liquidation_feed(message);

    assert_eq!(terminal.liquidations_status, "Current");
    assert!(terminal.liquidations_last_rx_ms.is_none());
    assert!(terminal.liquidations.is_empty());
    assert!(terminal.liquidation_summary_buckets.is_empty());
    assert!(terminal.liquidation_chart_buckets.is_empty());
}
