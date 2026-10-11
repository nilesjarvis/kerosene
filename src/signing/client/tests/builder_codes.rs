use super::{CapturedAgentKey, ExchangeOrderKind, PlaceOrderRequest, TEST_PRIVATE_KEY};
use crate::builder_codes::{BuilderCodeConfig, BuilderFee};
use crate::signing::client::{modify_order, place_order_with_cloid};
use zeroize::Zeroizing;

const ACCOUNT: &str = "0x1111111111111111111111111111111111111111";
const BUILDER: &str = "0xabcdef0123456789abcdef0123456789abcdef01";

fn request() -> PlaceOrderRequest {
    PlaceOrderRequest {
        asset: 7,
        is_buy: true,
        price: "123.45".to_string(),
        size: "0.25".to_string(),
        order_kind: ExchangeOrderKind::Limit,
        reduce_only: false,
        cloid: None,
    }
}

#[tokio::test]
async fn malformed_enabled_builder_rejects_before_signing_or_network() {
    // An invalid key proves the local configuration error is returned first.
    let key = CapturedAgentKey::new(Zeroizing::new("invalid-key".to_string()))
        .expect("nonempty key")
        .with_builder_config(
            &BuilderCodeConfig {
                enabled: true,
                ..BuilderCodeConfig::default()
            },
            ACCOUNT,
        );
    let response = place_order_with_cloid(key, request())
        .await
        .expect("definitive local response");
    assert!(response.is_error());
    assert!(!response.is_ambiguous_order_result());
    assert!(!response.has_potential_order_effect());
    assert!(response.summary().contains("Builder address"));
}

#[tokio::test]
async fn builder_subaccount_rejects_before_approval_or_order_submission() {
    let key =
        CapturedAgentKey::for_account(Zeroizing::new(TEST_PRIVATE_KEY.to_string()), Some(ACCOUNT))
            .expect("valid subaccount")
            .with_builder(
                Some(BuilderFee::new(BUILDER, 10).expect("valid builder")),
                ACCOUNT,
            )
            .expect("capture stays available for cancels");
    let response = place_order_with_cloid(key, request())
        .await
        .expect("definitive local response");
    assert!(response.is_error());
    assert!(!response.is_ambiguous_order_result());
    assert!(!response.has_potential_order_effect());
    assert!(response.summary().contains("subaccount"));
}

#[tokio::test]
async fn configured_builder_modification_is_definitive_local_rejection() {
    for config in [
        BuilderCodeConfig {
            enabled: true,
            address: BUILDER.to_string(),
            fee_tenths_bps: 10,
        },
        BuilderCodeConfig {
            enabled: true,
            ..BuilderCodeConfig::default()
        },
    ] {
        let key = CapturedAgentKey::new(Zeroizing::new("invalid-key".to_string()))
            .expect("nonempty key")
            .with_builder_config(&config, ACCOUNT);
        let response = modify_order(
            key,
            42,
            7,
            true,
            "123.45".to_string(),
            "0.25".to_string(),
            false,
        )
        .await
        .expect("definitive local response");
        assert!(response.is_error());
        assert!(!response.is_ambiguous_order_result());
        assert!(!response.has_potential_order_effect());
        assert!(response.summary().contains("Cancel and replace"));
    }
}

#[test]
fn local_rejection_redacts_sensitive_inputs_and_has_no_exchange_effect() {
    let response =
        crate::signing::ExchangeResponse::local_rejection(format!("Builder problem at {ACCOUNT}"));
    assert!(response.is_error());
    assert!(!response.is_ambiguous_order_result());
    assert!(!response.has_potential_order_effect());
    assert!(!response.summary().contains(ACCOUNT));
    assert!(!format!("{response:?}").contains(ACCOUNT));
}
