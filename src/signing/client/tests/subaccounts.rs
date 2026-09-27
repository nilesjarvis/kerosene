use super::{
    CapturedAgentKey, EXCHANGE_EXPIRES_AFTER_MS, ExchangeOrderKind, HyperliquidL1Action,
    TEST_PRIVATE_KEY, build_signed_exchange_payload_with_nonce,
    exchange_payload_contains_private_key,
};
use crate::signing::crypto::sign_l1_action;
use serde_json::{Value, json};
use zeroize::Zeroizing;

const SUBACCOUNT: &str = "0xabcdef0123456789abcdef0123456789abcdef01";
const OTHER_SUBACCOUNT: &str = "0xabcdef0123456789abcdef0123456789abcdef02";
const CLOID: &str = "0x11111111111111111111111111111111";
const NONCE: u64 = 1_700_000_000_000;

/// All action families must carry the captured target into both the signed
/// bytes and the posted envelope. Changing accounts cannot reuse a signature.
fn assert_account_targets(action: HyperliquidL1Action, expected_action: Value) {
    let packed = rmp_serde::to_vec_named(&action).expect("action should encode");
    let mut signatures = Vec::new();
    for target in [None, Some(SUBACCOUNT), Some(OTHER_SUBACCOUNT)] {
        let context =
            CapturedAgentKey::for_account(Zeroizing::new(TEST_PRIVATE_KEY.to_string()), target)
                .expect("valid account context");
        let payload =
            build_signed_exchange_payload_with_nonce(context.clone_for_task(), &action, NONCE)
                .expect("payload should sign without posting");

        assert_eq!(payload["action"], expected_action);
        assert_eq!(payload["nonce"], NONCE);
        assert_eq!(payload["expiresAfter"], NONCE + EXCHANGE_EXPIRES_AFTER_MS);
        assert_eq!(payload["vaultAddress"], json!(target));
        assert_eq!(
            payload["signature"],
            sign_l1_action(
                TEST_PRIVATE_KEY,
                &packed,
                target,
                NONCE,
                Some(NONCE + EXCHANGE_EXPIRES_AFTER_MS),
            )
            .expect("matching target should sign"),
        );
        assert!(!exchange_payload_contains_private_key(
            &payload,
            TEST_PRIVATE_KEY
        ));
        signatures.push(payload["signature"].clone());
    }
    assert_ne!(signatures[0], signatures[1]);
    assert_ne!(signatures[0], signatures[2]);
    assert_ne!(signatures[1], signatures[2]);
}

#[test]
fn order_payload_binds_master_or_subaccount_target() {
    assert_account_targets(
        HyperliquidL1Action::order_with_cloid(
            110_003,
            true,
            "2500".to_string(),
            "0.5".to_string(),
            ExchangeOrderKind::Limit,
            false,
            Some(CLOID.to_string()),
        ),
        json!({
            "type": "order",
            "orders": [{
                "a": 110_003, "b": true, "p": "2500", "s": "0.5", "r": false,
                "t": {"limit": {"tif": "Gtc"}}, "c": CLOID,
            }],
            "grouping": "na",
        }),
    );
}

#[test]
fn cancel_payload_binds_master_or_subaccount_target() {
    assert_account_targets(
        HyperliquidL1Action::cancel(110_003, 42),
        json!({"type": "cancel", "cancels": [{"a": 110_003, "o": 42}]}),
    );
}

#[test]
fn cancel_by_cloid_payload_binds_master_or_subaccount_target() {
    assert_account_targets(
        HyperliquidL1Action::cancel_by_cloid(110_003, CLOID.to_string()),
        json!({"type": "cancelByCloid", "cancels": [{"asset": 110_003, "cloid": CLOID}]}),
    );
}

#[test]
fn modify_payload_binds_master_or_subaccount_target() {
    assert_account_targets(
        HyperliquidL1Action::modify(
            42,
            110_003,
            false,
            "2500".to_string(),
            "0.5".to_string(),
            true,
        ),
        json!({
            "type": "batchModify",
            "modifies": [{"oid": 42, "order": {
                "a": 110_003, "b": false, "p": "2500", "s": "0.5", "r": true,
                "t": {"limit": {"tif": "Gtc"}},
            }}],
        }),
    );
}

#[test]
fn leverage_payload_binds_master_or_subaccount_target() {
    assert_account_targets(
        HyperliquidL1Action::update_leverage(110_003, true, 5),
        json!({"type": "updateLeverage", "asset": 110_003, "isCross": true, "leverage": 5}),
    );
}

#[test]
fn cleared_subaccount_context_cannot_sign_a_master_action() {
    let mut context = CapturedAgentKey::for_account(
        Zeroizing::new(TEST_PRIVATE_KEY.to_string()),
        Some(SUBACCOUNT),
    )
    .expect("valid subaccount context");
    context.clear();

    let error = build_signed_exchange_payload_with_nonce(
        context.clone_for_task(),
        &HyperliquidL1Action::cancel(110_003, 42),
        NONCE,
    )
    .expect_err("cleared key must fail before posting");

    assert!(error.contains("Invalid private key hex"));
    assert!(!error.contains(TEST_PRIVATE_KEY));
    assert!(!error.contains(SUBACCOUNT));
}
