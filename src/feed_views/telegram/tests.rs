use super::*;
use crate::api::{ExchangeSymbol, OutcomeSymbolInfo};

fn perp_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 50,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
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

fn post_with_mention(symbol: &str, ticker: &str) -> TelegramFeedPost {
    TelegramFeedPost {
        channel: "channel".to_string(),
        message_id: 1,
        text: "post".to_string(),
        timestamp_ms: 0,
        source: crate::telegram_feed::TelegramFeedPostSource::PublicPoll,
        received_at_ms: 0,
        applied_at_ms: 0,
        fetched_at_ms: 0,
        request_started_ms: 0,
        request_duration_ms: 0,
        first_seen_ms: 0,
        url: String::new(),
        ticker_mentions: vec![crate::telegram_feed::TelegramTickerMention {
            symbol: symbol.to_string(),
            ticker: ticker.to_string(),
            matched_text: ticker.to_string(),
            source: SymbolAliasSource::Keyword,
            confidence: 74,
            reference_price: None,
            reference_seen_ms: 0,
        }],
    }
}

fn spot_symbol(key: &str, ticker: &str) -> ExchangeSymbol {
    let mut symbol = perp_symbol(key);
    symbol.ticker = ticker.to_string();
    symbol.market_type = MarketType::Spot;
    symbol
}

#[test]
fn telegram_ticker_impact_cards_resolve_outcome_display_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(outcome_symbol("#950"));

    let cards = terminal.telegram_ticker_impact_cards(&post_with_mention("#950", "OUT95-YES"));

    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].ticker, "YES: Will BTC close green?");
    assert_eq!(cards[0].symbol, "#950");
}

#[test]
fn telegram_ticker_impact_cards_keep_perp_tickers() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(perp_symbol("HYPE"));

    let cards = terminal.telegram_ticker_impact_cards(&post_with_mention("HYPE", "HYPE"));

    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].ticker, "HYPE");
}

#[test]
fn telegram_ticker_impact_cards_compute_signed_impact_against_fresh_mid() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(perp_symbol("HYPE"));
    let now_ms = TradingTerminal::now_ms();
    terminal.all_mids.insert("HYPE".to_string(), 110.0);
    terminal
        .all_mids_updated_at_ms
        .insert("HYPE".to_string(), now_ms);

    let mut post = post_with_mention("HYPE", "HYPE");
    post.ticker_mentions[0].reference_price = Some(100.0);
    post.ticker_mentions[0].source = SymbolAliasSource::Ticker;

    // A rise vs the reference reads positive...
    let cards = terminal.telegram_ticker_impact_cards(&post);
    assert_eq!(cards.len(), 1);
    let impact = cards[0].impact_pct.expect("impact");
    assert!((impact - 10.0).abs() < 1e-9, "expected +10%, got {impact}");

    // ...and a fall reads negative (guards against an argument swap).
    terminal.all_mids.insert("HYPE".to_string(), 90.0);
    let cards = terminal.telegram_ticker_impact_cards(&post);
    let impact = cards[0].impact_pct.expect("impact");
    assert!((impact + 10.0).abs() < 1e-9, "expected -10%, got {impact}");

    // A stale mid yields no impact rather than a misleading 0%.
    terminal.all_mids.insert("HYPE".to_string(), 110.0);
    terminal
        .all_mids_updated_at_ms
        .insert("HYPE".to_string(), now_ms.saturating_sub(60_000));
    let cards = terminal.telegram_ticker_impact_cards(&post);
    assert_eq!(cards[0].impact_pct, None);
}

#[test]
fn telegram_ticker_impact_cards_drop_unorderable_and_spot_mentions() {
    let mut terminal = TradingTerminal::boot().0;

    // A mention whose symbol is no longer in the exchange list is dropped.
    let cards = terminal.telegram_ticker_impact_cards(&post_with_mention("GHOST", "GHOST"));
    assert!(cards.is_empty());

    // A spot symbol is dropped even when present.
    terminal.exchange_symbols.push(spot_symbol("@1", "PURR"));
    let cards = terminal.telegram_ticker_impact_cards(&post_with_mention("@1", "PURR"));
    assert!(cards.is_empty());
}
