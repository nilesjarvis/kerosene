use super::super::{
    TwapAccountRefresh, TwapExchangeErrorAction, classify_twap_exchange_error,
    twap_place_result_refresh_policy,
};
use super::fixtures::{exchange_response, exchange_response_from_value};
use crate::signing::ExchangeResponse;
use crate::twap_state::TwapPauseReason;

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
