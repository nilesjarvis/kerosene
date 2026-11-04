use super::*;

#[test]
fn position_tpsl_wire_uses_entire_position_reduce_only_and_sdk_field_order() {
    let action = HyperliquidL1Action::position_tpsl(vec![PositionTpslOrder {
        asset: 110_003,
        is_buy: false,
        price: "99".into(),
        trigger_px: "110".into(),
        kind: TpslKind::TakeProfit,
        cloid: "0x11111111111111111111111111111111".into(),
    }]);
    let expected = serde_json::json!({
        "type": "order", "orders": [{
            "a": 110003, "b": false, "p": "99", "s": "0", "r": true,
            "t": {"trigger": {"isMarket": true, "triggerPx": "110", "tpsl": "tp"}},
            "c": "0x11111111111111111111111111111111"
        }], "grouping": "positionTpsl"
    });
    assert_eq!(serde_json::to_value(&action).expect("JSON wire"), expected);
    let packed = rmp_serde::to_vec_named(&action).expect("msgpack wire");
    // Python msgpack.packb of the SDK's ordered wire dictionary, including
    // the isMarket/triggerPx/tpsl field order used by order_type_to_wire.
    let expected_msgpack = "83a474797065a56f72646572a66f72646572739187a161ce0001adb3a162c2a170a23939a173a130a172c3a17481a77472696767657283a869734d61726b6574c3a9747269676765725078a3313130a47470736ca27470a163d92230783131313131313131313131313131313131313131313131313131313131313131a867726f7570696e67ac706f736974696f6e5470736c";
    assert_eq!(hex::encode(&packed), expected_msgpack);
    let decoded: serde_json::Value = rmp_serde::from_slice(&packed).expect("msgpack map");
    assert_eq!(decoded, expected);
    let offsets: Vec<_> = [
        "type",
        "orders",
        "trigger",
        "isMarket",
        "triggerPx",
        "tpsl",
        "grouping",
    ]
    .iter()
    .map(|key| {
        packed
            .windows(key.len())
            .position(|bytes| bytes == key.as_bytes())
            .expect("signed field")
    })
    .collect();
    assert!(offsets.windows(2).all(|pair| pair[0] < pair[1]));
    let debug = format!("{action:?}");
    assert!(!debug.contains("110"));
    assert!(!debug.contains("0x111111"));
}
