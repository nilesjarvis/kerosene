use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;

use super::formatting::{
    TickerTapeItem, percent_change, percent_label, price_label, ticker_tape_item_width,
};
use super::{TICKER_TAPE_ITEM_MAX_WIDTH, TICKER_TAPE_ITEM_MIN_WIDTH};

// ---------------------------------------------------------------------------
// Ticker Tape Formatting Tests
// ---------------------------------------------------------------------------

fn outcome_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT95-YES".to_string(),
        category: "outcome".to_string(),
        display_name: Some("YES: Will BTC close green?".to_string()),
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
fn ticker_tape_outcome_favourite_uses_short_condition_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(outcome_symbol("#950"));
    terminal.favourite_symbols = vec!["#950".to_string()];

    let items = terminal.ticker_tape_items();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].symbol, "#950");
    assert_eq!(items[0].ticker, "Will BTC close green?");
}

#[test]
fn ticker_tape_unloaded_outcome_favourite_uses_cached_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.favourite_symbols = vec!["#950".to_string()];
    terminal
        .outcome_display_labels
        .insert("#950".to_string(), "YES: Will BTC close green?".to_string());

    let items = terminal.ticker_tape_items();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].ticker, "YES: Will BTC close green?");
}

#[test]
fn percent_change_preserves_signed_relative_move() {
    assert_eq!(percent_change(Some(110.0), Some(100.0)), Some(10.0));
    assert_eq!(percent_change(Some(75.0), Some(100.0)), Some(-25.0));
}

#[test]
fn percent_change_requires_positive_current_and_previous_price() {
    assert_eq!(percent_change(None, Some(100.0)), None);
    assert_eq!(percent_change(Some(100.0), None), None);
    assert_eq!(percent_change(Some(0.0), Some(100.0)), None);
    assert_eq!(percent_change(Some(100.0), Some(0.0)), None);
    assert_eq!(percent_change(Some(f64::NAN), Some(100.0)), None);
    assert_eq!(percent_change(Some(100.0), Some(f64::INFINITY)), None);
}

#[test]
fn ticker_tape_labels_use_existing_placeholder_and_sign_conventions() {
    let denomination = DisplayDenominationContext::usd();

    assert_eq!(price_label(None, &denomination), "-");
    assert_eq!(percent_label(None), "-");
    assert_eq!(percent_label(Some(1.234)), "+1.23%");
    assert_eq!(percent_label(Some(-1.234)), "-1.23%");
}

#[test]
fn ticker_tape_item_width_stays_within_layout_bounds() {
    let denomination = DisplayDenominationContext::usd();
    let narrow_item = TickerTapeItem {
        symbol: "BTC".to_string(),
        ticker: "BTC".to_string(),
        price: None,
        pct_24h: None,
    };
    let wide_item = TickerTapeItem {
        symbol: "VERY-LONG-SYMBOL-NAME".to_string(),
        ticker: "VERY-LONG-SYMBOL-NAME".to_string(),
        price: Some(123_456.789),
        pct_24h: Some(123.456),
    };

    assert_eq!(
        ticker_tape_item_width(&narrow_item, &denomination),
        TICKER_TAPE_ITEM_MIN_WIDTH
    );
    assert_eq!(
        ticker_tape_item_width(&wide_item, &denomination),
        TICKER_TAPE_ITEM_MAX_WIDTH
    );
}
