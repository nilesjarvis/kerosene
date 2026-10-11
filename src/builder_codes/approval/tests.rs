use super::*;

const ACCOUNT: &str = "0x1111111111111111111111111111111111111111";
const BUILDER: &str = "0x2222222222222222222222222222222222222222";

#[test]
fn approval_payload_matches_hyperliquid_user_signed_schema_and_exact_fee() {
    let builder = BuilderFee::new(BUILDER, 13).expect("valid builder");
    let config = approval_config(ACCOUNT, &builder, 1_700_000_000_123, "/session/complete");
    assert_eq!(config["account"], ACCOUNT);
    assert_eq!(config["feeBps"], "1.3");
    assert_eq!(
        config["action"],
        json!({
            "type": "approveBuilderFee",
            "hyperliquidChain": "Mainnet",
            "maxFeeRate": "0.013%",
            "builder": BUILDER,
            "nonce": 1_700_000_000_123_u64,
        })
    );
    assert_eq!(
        config["domain"],
        json!({
            "name": "HyperliquidSignTransaction",
            "version": "1",
            "verifyingContract": "0x0000000000000000000000000000000000000000",
        })
    );
    assert_eq!(
        config["primaryType"],
        "HyperliquidTransaction:ApproveBuilderFee"
    );
    assert_eq!(
        config["types"]["HyperliquidTransaction:ApproveBuilderFee"],
        json!([
            {"name": "hyperliquidChain", "type": "string"},
            {"name": "maxFeeRate", "type": "string"},
            {"name": "builder", "type": "address"},
            {"name": "nonce", "type": "uint64"},
        ])
    );
    assert_eq!(config["expiresAt"], 1_700_000_300_123_u64);
}

#[test]
fn script_serialization_cannot_terminate_script_or_create_markup() {
    let value = json!({"name": "</script><script>&\u{2028}\u{2029}"});
    let encoded = script_json(&value);
    assert!(!encoded.contains(['<', '>', '&', '\u{2028}', '\u{2029}']));
    assert_eq!(
        serde_json::from_str::<Value>(&encoded).expect("valid JSON"),
        value
    );
}

#[test]
fn session_renders_immutable_values_and_private_completion_path() {
    let builder = BuilderFee::new(BUILDER, 1).expect("valid builder");
    let session = ApprovalSession::new(
        "127.0.0.1:43210".into(),
        "a".repeat(64),
        ACCOUNT,
        &builder,
        1,
    );
    assert_eq!(session.origin, "http://127.0.0.1:43210");
    assert_eq!(
        session.completion_path,
        format!("/{}/complete", "a".repeat(64))
    );
    assert!(!session.html.contains("__KEROSENE_"));
    assert!(session.html.contains("\"maxFeeRate\":\"0.001%\""));
    assert!(
        session
            .html
            .contains("\"account\":\"0x1111111111111111111111111111111111111111\"")
    );
}

#[tokio::test]
async fn invalid_account_is_rejected_before_starting_a_browser_session() {
    let builder = BuilderFee::new(BUILDER, 1).expect("valid builder");
    assert!(
        approve_in_wallet("not an address".into(), builder)
            .await
            .is_err()
    );
}
