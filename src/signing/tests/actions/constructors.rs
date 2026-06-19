use super::{
    ExchangeOrderKind, HyperliquidL1Action, build_cancel_action, build_modify_action,
    build_order_action, build_update_leverage_action, json_value, msgpack_named,
};

#[test]
fn action_enum_order_constructor_round_trips_through_existing_builder() {
    let direct = build_order_action(
        7,
        true,
        "123.45".to_string(),
        "0.25".to_string(),
        ExchangeOrderKind::Limit,
        false,
    );
    let via_enum = HyperliquidL1Action::order(
        7,
        true,
        "123.45".to_string(),
        "0.25".to_string(),
        ExchangeOrderKind::Limit,
        false,
    );

    let direct_json = json_value(&direct, "direct action should serialize");
    let via_enum_json = json_value(&via_enum, "enum action should serialize");
    assert_eq!(direct_json, via_enum_json);

    let direct_msgpack = msgpack_named(&direct, "direct action should encode as msgpack");
    let via_enum_msgpack = msgpack_named(&via_enum, "enum action should encode as msgpack");
    assert_eq!(
        direct_msgpack, via_enum_msgpack,
        "wire bytes must match; the action hash depends on them"
    );
}

#[test]
fn action_enum_cancel_constructor_matches_direct_builder() {
    let direct = build_cancel_action(3, 9001);
    let via_enum = HyperliquidL1Action::cancel(3, 9001);

    assert_eq!(
        msgpack_named(&direct, "direct cancel should encode as msgpack"),
        msgpack_named(&via_enum, "enum cancel should encode as msgpack"),
    );
}

#[test]
fn action_enum_modify_constructor_matches_direct_builder() {
    let direct = build_modify_action(
        9001,
        3,
        true,
        "123.45".to_string(),
        "0.25".to_string(),
        false,
    );
    let via_enum = HyperliquidL1Action::modify(
        9001,
        3,
        true,
        "123.45".to_string(),
        "0.25".to_string(),
        false,
    );

    assert_eq!(
        msgpack_named(&direct, "direct modify should encode as msgpack"),
        msgpack_named(&via_enum, "enum modify should encode as msgpack"),
    );
}

#[test]
fn action_enum_update_leverage_constructor_matches_direct_builder() {
    let direct = build_update_leverage_action(110_003, false, 7);
    let via_enum = HyperliquidL1Action::update_leverage(110_003, false, 7);

    let direct_json = json_value(&direct, "direct leverage action should serialize");
    let via_enum_json = json_value(&via_enum, "enum leverage action should serialize");
    assert_eq!(direct_json, via_enum_json);

    assert_eq!(
        msgpack_named(&direct, "direct leverage should encode as msgpack"),
        msgpack_named(&via_enum, "enum leverage should encode as msgpack"),
    );
}

#[test]
fn action_enum_debug_uses_redacted_inner_actions() {
    let cloid = "0x1234567890abcdef1234567890abcdef".to_string();
    let actions = [
        HyperliquidL1Action::order_with_cloid(
            7,
            true,
            "price-secret".to_string(),
            "size-secret".to_string(),
            ExchangeOrderKind::Limit,
            false,
            Some(cloid.clone()),
        ),
        HyperliquidL1Action::cancel(7, 9001),
        HyperliquidL1Action::cancel_by_cloid(7, cloid.clone()),
        HyperliquidL1Action::modify(
            9001,
            7,
            true,
            "price-secret".to_string(),
            "size-secret".to_string(),
            false,
        ),
    ];

    for action in actions {
        let rendered = format!("{action:?}");

        assert!(rendered.contains("Action"));
        assert!(!rendered.contains("price-secret"));
        assert!(!rendered.contains("size-secret"));
        assert!(!rendered.contains("9001"));
        assert!(!rendered.contains(&cloid));
    }
}
