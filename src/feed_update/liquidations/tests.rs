use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::LiquidationEvent;

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
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 95,
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

    let _ = terminal.update_liquidation_feed(Message::WsHydromancerLiquidation(
        crate::ws::HydromancerWsMessage::Event(liquidation),
    ));

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

    let _ = terminal.update_liquidation_feed(Message::WsHydromancerLiquidation(
        crate::ws::HydromancerWsMessage::Event(liquidation),
    ));
    assert!(!terminal.liquidations.is_empty());
    assert!(!terminal.liquidation_summary_buckets.is_empty());
    assert!(!terminal.liquidation_chart_buckets.is_empty());

    let _ = terminal.update_liquidation_feed(Message::ClearLiquidations);

    assert!(terminal.liquidations.is_empty());
    assert!(terminal.liquidation_summary_buckets.is_empty());
    assert!(terminal.liquidation_chart_buckets.is_empty());
}
