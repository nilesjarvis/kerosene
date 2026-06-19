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
fn nuke_planning_debug_redacts_order_and_position_details() {
    let order = NukePositionOrder {
        asset: 9,
        is_buy: true,
        price: "price-secret".to_string(),
        size: "size-secret".to_string(),
    };
    let input = nuke_input(
        "SECRETCOIN",
        "raw-size-secret",
        true,
        Some(perp_sym()),
        Some(12345.67),
    );
    let classification = NukePositionClassification::Order(order.clone());
    let plan = NukePlan {
        ready: vec![("SECRETCOIN".to_string(), order.clone())],
        skipped: vec![("SKIPPEDCOIN".to_string(), NukeSkipReason::NoMidPrice)],
        hidden_skipped: vec![("HIDDENCOIN".to_string(), NukeSkipReason::UnknownAsset)],
    };

    let order_debug = format!("{order:?}");
    let input_debug = format!("{input:?}");
    let classification_debug = format!("{classification:?}");
    let plan_debug = format!("{plan:?}");

    assert!(order_debug.contains("asset: 9"));
    assert!(order_debug.contains("is_buy: true"));
    assert!(!order_debug.contains("price-secret"));
    assert!(!order_debug.contains("size-secret"));

    assert!(input_debug.contains("is_hidden: true"));
    assert!(!input_debug.contains("SECRETCOIN"));
    assert!(!input_debug.contains("raw-size-secret"));
    assert!(!input_debug.contains("12345.67"));

    assert!(classification_debug.contains("NukePositionOrder"));
    assert!(!classification_debug.contains("price-secret"));
    assert!(!classification_debug.contains("size-secret"));

    assert!(plan_debug.contains("ready_count: 1"));
    assert!(plan_debug.contains("skipped_count: 1"));
    assert!(plan_debug.contains("hidden_skipped_count: 1"));
    assert!(!plan_debug.contains("SECRETCOIN"));
    assert!(!plan_debug.contains("SKIPPEDCOIN"));
    assert!(!plan_debug.contains("HIDDENCOIN"));
    assert!(!plan_debug.contains("price-secret"));
    assert!(!plan_debug.contains("size-secret"));
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
