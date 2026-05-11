use super::*;

fn open_order(coin: &str) -> OpenOrder {
    OpenOrder {
        coin: coin.to_string(),
        side: "B".to_string(),
        limit_px: "10".to_string(),
        sz: "1".to_string(),
        oid: 1,
        timestamp: 1,
        reduce_only: Some(false),
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
fn hip3_bootstrap_open_order_symbols_are_normalized() {
    let mut orders = vec![open_order("BTC"), open_order("flx:ETH")];

    normalize_dex_open_order_coins("flx", &mut orders);

    assert_eq!(orders[0].coin, "flx:BTC");
    assert_eq!(orders[1].coin, "flx:ETH");
}
