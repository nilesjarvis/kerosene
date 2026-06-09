use super::*;

#[test]
fn plan_skip_list_renders_each_position_with_its_reason() {
    let plan = NukePlan {
        ready: vec![],
        skipped: vec![
            ("SHIB".to_string(), NukeSkipReason::NoMidPrice),
            ("BTC-SPOT".to_string(), NukeSkipReason::NonPerp),
        ],
        hidden_skipped: vec![],
    };
    assert_eq!(
        plan.format_skip_list(),
        "SHIB (no mid price), BTC-SPOT (not a perpetual market)"
    );
}

#[test]
fn plan_ready_list_renders_each_coin_in_order() {
    let order = order_or_panic(
        build_nuke_position_order(1, 2, 100.0, 1.0, DEFAULT_MARKET_SLIPPAGE),
        "order",
    );
    let plan = NukePlan {
        ready: vec![
            ("BTC".to_string(), order.clone()),
            ("ETH".to_string(), order.clone()),
            ("SOL".to_string(), order),
        ],
        skipped: vec![],
        hidden_skipped: vec![],
    };
    assert_eq!(plan.format_ready_list(), "BTC, ETH, SOL");
}

#[test]
fn plan_is_empty_iff_both_lists_are_empty() {
    assert!(NukePlan::default().is_empty());
    let with_skip = NukePlan {
        ready: vec![],
        skipped: vec![("X".to_string(), NukeSkipReason::UnknownAsset)],
        hidden_skipped: vec![],
    };
    assert!(!with_skip.is_empty());
}
