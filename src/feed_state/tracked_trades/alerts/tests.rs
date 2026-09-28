use super::*;
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};

fn trade_event() -> TrackedTradeEvent {
    TrackedTradeEvent {
        address: "synthetic-address".to_string(),
        coin: "HYPE".to_string(),
        price: 10.0,
        size: 1.0,
        is_buy: true,
        time_ms: 20_000,
        dir: "Open Long".to_string(),
        start_position: Some(0.0),
        closed_pnl: 0.0,
        fee: 0.01,
        fee_token: "USDC".to_string(),
        tid: Some(7),
        hash: "synthetic-hash".to_string(),
        oid: Some(9),
        tx_index: 3,
    }
}

#[test]
fn alert_suppression_preserves_order_hash_and_time_window_rules() {
    for (oid, hash, gap, suppressed) in [
        (Some(9), "synthetic-hash", 10_000, true),
        (None, "synthetic-hash", 10_000, true),
        (None, "", 500, true),
        (None, "", 501, false),
        (None, "  ", 500, true),
        (None, "  ", 501, false),
    ] {
        for older in [false, true] {
            let mut incoming = trade_event();
            incoming.oid = oid;
            incoming.hash = hash.to_string();
            let mut existing = incoming.clone();
            existing.time_ms = if older {
                incoming.time_ms - gap
            } else {
                incoming.time_ms + gap
            };
            let history = VecDeque::from([existing]);

            let alert =
                TradingTerminal::tracked_trade_alert_row_for_event_from(&history, true, &incoming);
            assert_eq!(alert.is_none(), suppressed);
            let ungrouped =
                TradingTerminal::tracked_trade_alert_row_for_event_from(&history, false, &incoming)
                    .expect("ungrouped fills should always produce an alert row");
            assert_eq!(ungrouped.fill_count, 1);
            assert_eq!(ungrouped.first_time_ms, incoming.time_ms);
            assert_eq!(ungrouped.notional, 10.0);
        }
    }
}

#[test]
fn alert_suppression_requires_matching_identity_and_searches_past_unrelated_events() {
    let incoming = trade_event();
    for unrelated in [
        TrackedTradeEvent {
            address: "other-address".to_string(),
            ..incoming.clone()
        },
        TrackedTradeEvent {
            coin: "BTC".to_string(),
            ..incoming.clone()
        },
        TrackedTradeEvent {
            is_buy: false,
            ..incoming.clone()
        },
        TrackedTradeEvent {
            oid: Some(10),
            ..incoming.clone()
        },
    ] {
        let mut history = VecDeque::from([unrelated]);
        assert!(
            TradingTerminal::tracked_trade_alert_row_for_event_from(&history, true, &incoming)
                .is_some()
        );
        history.push_back(incoming.clone());
        assert!(
            TradingTerminal::tracked_trade_alert_row_for_event_from(&history, true, &incoming)
                .is_none()
        );
    }

    let mut incoming = incoming;
    incoming.oid = None;
    let different_hash = TrackedTradeEvent {
        hash: "other-hash".to_string(),
        ..incoming.clone()
    };
    assert!(
        TradingTerminal::tracked_trade_alert_row_for_event_from(
            &VecDeque::from([different_hash]),
            true,
            &incoming
        )
        .is_some()
    );
    incoming.hash.clear();
    let different_direction = TrackedTradeEvent {
        dir: "Close Long".to_string(),
        ..incoming.clone()
    };
    assert!(
        TradingTerminal::tracked_trade_alert_row_for_event_from(
            &VecDeque::from([different_direction]),
            true,
            &incoming
        )
        .is_some()
    );
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
