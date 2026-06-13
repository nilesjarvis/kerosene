use super::responses::{fee_rates_from_best_effort_value, record_best_effort_section_warnings};
use super::{
    FUNDING_HISTORY_LOOKBACK_MS, frontend_open_orders_payload, funding_history_start_ms_from,
    user_fills_payload,
};
use crate::account::{
    AccountDataCompleteness, AccountDataSection, OpenOrder, UserFeeRates,
    normalize_dex_open_order_coins,
};

fn open_order(coin: &str) -> OpenOrder {
    OpenOrder {
        coin: coin.to_string(),
        side: "B".to_string(),
        limit_px: "10".to_string(),
        sz: "1".to_string(),
        oid: 1,
        timestamp: 1,
        reduce_only: Some(false),
        is_trigger: None,
        order_type: None,
        tif: None,
        trigger_px: None,
    }
}

#[test]
fn fee_rate_parse_failure_marks_fees_incomplete() {
    let mut completeness = AccountDataCompleteness::default();
    let rates = fee_rates_from_best_effort_value(
        Err("userFees parse failed: invalid json".to_string()),
        &mut completeness,
    );

    assert_eq!(
        rates.user_cross_rate,
        UserFeeRates::default().user_cross_rate
    );
    assert_eq!(
        completeness.section_warning(AccountDataSection::Fees),
        Some("Fee rates may be incomplete: userFees parse failed: invalid json".to_string())
    );
}

#[test]
fn fee_rate_parse_success_keeps_fees_complete() {
    let mut completeness = AccountDataCompleteness::default();
    let rates = fee_rates_from_best_effort_value(
        Ok(serde_json::json!({
            "userCrossRate": "0.0004",
            "userAddRate": "0.0001"
        })),
        &mut completeness,
    );

    assert!(rates.rate_for(false, false).is_some());
    assert_eq!(completeness.section_warning(AccountDataSection::Fees), None);
}

#[test]
fn funding_history_start_uses_seven_day_lookback() {
    assert_eq!(
        funding_history_start_ms_from(FUNDING_HISTORY_LOOKBACK_MS + 42),
        42
    );
}

#[test]
fn funding_history_start_saturates_before_lookback_window() {
    assert_eq!(funding_history_start_ms_from(42), 0);
}

#[test]
fn selected_hip3_refresh_scopes_open_orders_but_keeps_user_fills_account_wide() {
    let address = "0xabc0000000000000000000000000000000000000";

    assert_eq!(
        frontend_open_orders_payload(address, Some("flx")),
        serde_json::json!({
            "type": "frontendOpenOrders",
            "user": address,
            "dex": "flx"
        })
    );
    assert_eq!(
        frontend_open_orders_payload(address, None),
        serde_json::json!({
            "type": "frontendOpenOrders",
            "user": address
        })
    );

    let fills_payload = user_fills_payload(address);
    assert_eq!(
        fills_payload,
        serde_json::json!({
            "type": "userFills",
            "user": address
        })
    );
    assert!(
        fills_payload.get("dex").is_none(),
        "userFills is account-wide; do not add unsupported selected-dex scoping"
    );
}

#[test]
fn best_effort_warnings_mark_matching_sections() {
    let mut completeness = AccountDataCompleteness::default();

    record_best_effort_section_warnings(
        &mut completeness,
        vec![
            "frontendOpenOrders request failed".to_string(),
            "userFills parse failed".to_string(),
            "other bootstrap warning".to_string(),
        ],
    );

    assert_eq!(
        completeness.section_warning(AccountDataSection::OpenOrders),
        Some("Open orders may be incomplete: frontendOpenOrders request failed".to_string())
    );
    assert_eq!(
        completeness.section_warning(AccountDataSection::Fills),
        Some("Trade history may be incomplete: userFills parse failed".to_string())
    );
    assert_eq!(
        completeness.section_warning(AccountDataSection::Positions),
        Some("Positions may be incomplete: other bootstrap warning".to_string())
    );
}

#[test]
fn hip3_bootstrap_open_order_symbols_are_normalized() {
    let mut orders = vec![open_order("BTC"), open_order("flx:ETH")];

    normalize_dex_open_order_coins("flx", &mut orders);

    assert_eq!(orders[0].coin, "flx:BTC");
    assert_eq!(orders[1].coin, "flx:ETH");
}
