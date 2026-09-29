use super::super::{
    TwapAccountRefresh, TwapExchangeErrorAction, classify_twap_exchange_error,
    twap_place_result_refresh_policy,
};
use super::fixtures::{exchange_response, exchange_response_from_value, pending_twap, twap_by_id};
use crate::app_state::TradingTerminal;
use crate::signing::ExchangeResponse;
use crate::twap_state::{TwapChildStatus, TwapPauseReason, TwapStatus};
use std::time::Instant;

mod ownership;

#[test]
fn twap_place_refresh_policy_reconciles_only_unknown_or_terminal_results() {
    let unknown: Result<ExchangeResponse, String> =
        Err("Exchange request failed after submit".to_string());
    assert_eq!(
        twap_place_result_refresh_policy(&unknown),
        TwapAccountRefresh::Immediate
    );

    let rejected = Ok(exchange_response(serde_json::json!({
        "error": "Order must have minimum value of $10"
    })));
    assert_eq!(
        twap_place_result_refresh_policy(&rejected),
        TwapAccountRefresh::None
    );

    let filled = Ok(exchange_response(serde_json::json!({
        "filled": {
            "totalSz": "1.25",
            "avgPx": "100",
            "oid": 77_u64
        }
    })));
    assert_eq!(
        twap_place_result_refresh_policy(&filled),
        TwapAccountRefresh::OnTerminal
    );

    let ambiguous: Result<ExchangeResponse, String> = Ok(exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": "schema-shifted"
                }
            }
        }),
        "ambiguous exchange response should deserialize",
    ));
    assert_eq!(
        twap_place_result_refresh_policy(&ambiguous),
        TwapAccountRefresh::Immediate
    );

    assert!(!TwapAccountRefresh::OnTerminal.should_refresh(false));
    assert!(TwapAccountRefresh::OnTerminal.should_refresh(true));
    assert!(TwapAccountRefresh::Immediate.should_refresh(false));
}

#[test]
fn twap_exchange_error_classification_separates_retryable_and_terminal_errors() {
    assert_eq!(
        classify_twap_exchange_error("Error: 429 Too Many Requests"),
        TwapExchangeErrorAction::Retry(TwapPauseReason::RateLimited)
    );
    assert_eq!(
        classify_twap_exchange_error("Error: Order must have minimum value of $10"),
        TwapExchangeErrorAction::Terminal
    );
    assert_eq!(
        classify_twap_exchange_error("Error: Order could not immediately match"),
        TwapExchangeErrorAction::ConsumeSlice
    );
}

#[test]
fn retryable_slice_error_pauses_active_twap_for_retry() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal
        .twap_orders
        .insert(1, pending_twap(1, "0xaaa", now));

    let _task = terminal.handle_twap_slice_result(
        1,
        Ok(exchange_response(serde_json::json!({
            "error": "429 Too Many Requests"
        }))),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Paused);
    assert_eq!(twap.pause_reason, Some(TwapPauseReason::RateLimited));
    assert_eq!(twap.pending_op, None);
    assert!(twap.retry_slice.is_some());
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Retrying);
}

#[test]
fn twap_exchange_error_classification_preserves_priority_and_matching() {
    for phrase in [
        "rate limit",
        "ratelimit",
        "too many requests",
        "429",
        "temporarily",
        "unavailable",
        "overloaded",
        "try again",
    ] {
        for summary in [
            phrase.to_string(),
            phrase.to_ascii_uppercase(),
            format!("prefix{phrase}suffix"),
            format!("Invalid signature; insufficient margin; {phrase}"),
        ] {
            assert_eq!(
                classify_twap_exchange_error(&summary),
                TwapExchangeErrorAction::Retry(TwapPauseReason::RateLimited),
                "{summary}"
            );
        }
    }
    for phrase in [
        "signature",
        "agent",
        "unauthorized",
        "not approved",
        "minimum",
        "min trade",
        "notional",
        "tick",
        "insufficient",
        "margin",
        "balance",
        "reduce only",
        "reduce-only",
        "open interest",
        "oracle",
        "delist",
        "max position",
    ] {
        assert_eq!(
            classify_twap_exchange_error(&phrase.to_ascii_uppercase()),
            TwapExchangeErrorAction::Terminal,
            "{phrase}"
        );
    }
    for summary in [
        "",
        "timeout",
        "order could not immediately match",
        "rate  limit",
        "too many\nrequests",
        "try-again",
        "temporarİly",
        "overloaⅾed",
    ] {
        assert_eq!(
            classify_twap_exchange_error(summary),
            TwapExchangeErrorAction::ConsumeSlice,
            "{summary}"
        );
    }
}

#[test]
fn stopped_in_flight_twap_does_not_retry_after_retryable_slice_error() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal
        .twap_orders
        .insert(1, pending_twap(1, "0xaaa", now));

    let _task = terminal.stop_twap(1);

    let _task = terminal.handle_twap_slice_result(
        1,
        Ok(exchange_response(serde_json::json!({
            "error": "429 Too Many Requests"
        }))),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Stopped);
    assert_eq!(twap.pending_op, None);
    assert_eq!(twap.retry_slice, None);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::NoFill);
    assert!(
        twap.child_orders[0]
            .exchange_summary
            .contains("429 Too Many Requests")
    );
    assert_eq!(
        terminal
            .order_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some(("TWAP stopped", false))
    );
    assert!(
        terminal
            .advanced_order_history
            .iter()
            .any(|entry| entry.source_id == 1 && entry.status == "Stopped")
    );
}
