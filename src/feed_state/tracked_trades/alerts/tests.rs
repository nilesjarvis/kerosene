use super::*;
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};

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

fn row_with_coin(coin: &str) -> TrackedTradeFeedRow {
    TrackedTradeFeedRow {
        address: "0x0000000000000000000000000000000000000001".to_string(),
        coin: coin.to_string(),
        is_buy: true,
        first_time_ms: 0,
        last_time_ms: 0,
        size: 10.0,
        notional: 5.0,
        avg_price: 0.5,
        closed_pnl: 0.0,
        fee: 0.0,
        fee_token: "USDC".to_string(),
        dir: String::new(),
        fill_count: 1,
        start_position: None,
        intent: TrackedTradeIntent::Opening,
        hash: String::new(),
        oid: None,
    }
}

#[test]
fn tracked_trade_alert_message_resolves_outcome_coin_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(outcome_symbol("#950"));

    let message = terminal.tracked_trade_alert_message_for_row(&row_with_coin("#950"));

    assert!(message.contains("YES: Will BTC close green?"), "{message}");
    assert!(!message.contains("#950"), "{message}");
}

#[test]
fn tracked_trade_alert_message_keeps_perp_tickers_uppercase() {
    let terminal = TradingTerminal::boot().0;

    let message = terminal.tracked_trade_alert_message_for_row(&row_with_coin("hype"));

    assert!(message.contains("HYPE"), "{message}");
}
