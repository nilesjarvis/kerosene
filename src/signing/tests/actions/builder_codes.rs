use super::{ExchangeOrderKind, HyperliquidL1Action, msgpack_named};
use crate::builder_codes::BuilderFee;
use crate::signing::crypto::action_hash_bytes;

const BUILDER: &str = "0xabcdef0123456789abcdef0123456789abcdef01";

fn order() -> HyperliquidL1Action {
    HyperliquidL1Action::order(
        7,
        true,
        "123.45".to_string(),
        "0.25".to_string(),
        ExchangeOrderKind::Limit,
        false,
    )
}

#[test]
fn builder_order_wire_and_hash_match_python_msgpack_fixtures() {
    // Generated independently with Python msgpack.packb and Cryptodome's Keccak,
    // using hyperliquid-python-sdk/utils/signing.py field and preimage order.
    let fee = BuilderFee::new(BUILDER, 37).expect("valid builder");
    let action = order().with_builder(Some(&fee));
    let bytes = msgpack_named(&action, "builder action encodes");
    assert_eq!(
        hex::encode(&bytes),
        "84a474797065a56f72646572a66f72646572739186a16107a162c3a170a63132332e3435a173a4302e3235a172c2a17481a56c696d697481a3746966a3477463a867726f7570696e67a26e61a76275696c64657282a162d92a307861626364656630313233343536373839616263646566303132333435363738396162636465663031a16625"
    );
    for (expiry, expected) in [
        (
            None,
            "1685243b0399d34115efd5458e2ad32bf27790250d44e5476cf261cdebcec535",
        ),
        (
            Some(1_700_000_030_000),
            "b71f8a060f21079eed582e7438c7860e62ec24c70340e41e0c8f1ceafa19a939",
        ),
    ] {
        let hash =
            action_hash_bytes(&bytes, None, 1_700_000_000_000, expiry).expect("hash succeeds");
        assert_eq!(hex::encode(hash), expected);
    }
    let json = serde_json::to_value(&action).expect("builder action serializes");
    assert_eq!(json["builder"], serde_json::json!({"b": BUILDER, "f": 37}));
    assert!(json["orders"][0].get("builder").is_none());
    let debug = format!("{action:?}");
    assert!(debug.contains("has_builder: true"));
    assert!(!debug.contains(BUILDER));
}

#[test]
fn disabled_builder_preserves_original_wire_and_hash() {
    let action = order().with_builder(None);
    let bytes = msgpack_named(&action, "disabled builder action encodes");
    assert_eq!(
        hex::encode(&bytes),
        "83a474797065a56f72646572a66f72646572739186a16107a162c3a170a63132332e3435a173a4302e3235a172c2a17481a56c696d697481a3746966a3477463a867726f7570696e67a26e61"
    );
    assert_eq!(
        hex::encode(
            action_hash_bytes(&bytes, None, 1_700_000_000_000, Some(1_700_000_030_000))
                .expect("hash succeeds")
        ),
        "9b50ded4bef8020d8a6abde1bd4213cf297a6c5b12ab53bf6319df04c10b5b2b"
    );
    assert!(
        serde_json::to_value(&action)
            .expect("action serializes")
            .get("builder")
            .is_none()
    );
}

#[test]
fn builder_cannot_be_attached_to_non_order_actions() {
    let builder = BuilderFee::new(BUILDER, 37).expect("valid builder");
    for action in [
        HyperliquidL1Action::cancel(7, 42),
        HyperliquidL1Action::cancel_by_cloid(7, "0x1234567890abcdef1234567890abcdef".to_string()),
        HyperliquidL1Action::modify(42, 7, true, "123.45".to_string(), "0.25".to_string(), false),
        HyperliquidL1Action::update_leverage(7, true, 5),
    ] {
        let original = msgpack_named(&action, "action encodes");
        assert_eq!(
            original,
            msgpack_named(&action.with_builder(Some(&builder)), "action encodes")
        );
    }
}
