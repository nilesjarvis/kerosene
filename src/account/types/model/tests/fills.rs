use super::*;

fn user_fill_value_with_coin_and_side(coin: &str, side: &str) -> serde_json::Value {
    let mut fill = user_fill_value_with_oid(Some(42));
    let Some(fill_obj) = fill.as_object_mut() else {
        panic!("test fill should serialize as object");
    };
    fill_obj.insert("coin".to_string(), serde_json::json!(coin));
    fill_obj.insert("side".to_string(), serde_json::json!(side));
    fill
}

#[test]
fn user_fill_preserves_optional_order_id_metadata() {
    let fill = user_fill_or_panic(user_fill_value_with_oid(Some(42)));

    assert_eq!(fill.oid, Some(42));
}

#[test]
fn user_fill_deserialization_preserves_canonical_market_symbols_and_wire_sides() {
    for (coin, side) in [("BTC", "B"), ("flx:BTC", "B"), ("@107", "A"), ("#950", "A")] {
        let fill = user_fill_or_panic(user_fill_value_with_coin_and_side(coin, side));

        assert_eq!(fill.coin, coin);
        assert_eq!(fill.side, side);
    }
}

#[test]
fn user_fill_accepts_missing_order_id_metadata() {
    let fill = user_fill_or_panic(user_fill_value_with_oid(None));

    assert_eq!(fill.oid, None);
}

#[test]
fn user_fill_accepts_optional_stable_identity_metadata() {
    let fill = user_fill_or_panic(user_fill_value_with_identity(123, "0xabc"));

    assert_eq!(fill.tid, Some(123));
    assert_eq!(fill.hash.as_deref(), Some("0xabc"));
}

#[test]
fn user_fill_debug_redacts_trade_payload() {
    let fill = UserFill {
        coin: "SECRETFILLCOIN".to_string(),
        px: "fill-price-secret".to_string(),
        sz: "fill-size-secret".to_string(),
        side: "A".to_string(),
        time: 123,
        hash: Some("fill-hash-secret".to_string()),
        tid: Some(777777),
        oid: Some(424242),
        dir: "Close Long".to_string(),
        closed_pnl: "fill-pnl-secret".to_string(),
        fee: "fill-fee-secret".to_string(),
    };

    let rendered = format!("{fill:?}");

    assert!(rendered.contains("UserFill"));
    assert!(rendered.contains("side: \"A\""));
    assert!(rendered.contains("time: 123"));
    assert!(rendered.contains("has_hash: true"));
    assert!(rendered.contains("has_tid: true"));
    assert!(rendered.contains("has_oid: true"));
    assert!(rendered.contains("dir: \"Close Long\""));
    for secret in [
        "SECRETFILLCOIN",
        "fill-price-secret",
        "fill-size-secret",
        "fill-hash-secret",
        "777777",
        "424242",
        "fill-pnl-secret",
        "fill-fee-secret",
    ] {
        assert!(
            !rendered.contains(secret),
            "user fill Debug leaked {secret}"
        );
    }
}

#[test]
fn user_fill_hash_dedup_key_includes_fill_fields_without_tid() {
    let mut first = user_fill_or_panic(user_fill_value_with_identity(123, "0xabc"));
    let mut second = first.clone();
    first.tid = None;
    second.tid = None;
    second.px = "101".to_string();

    assert_ne!(first.dedup_key(), second.dedup_key());
}
