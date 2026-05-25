use super::info_request_payload;

#[test]
fn info_request_payload_uses_requested_type() {
    assert_eq!(
        info_request_payload("spotMeta"),
        serde_json::json!({ "type": "spotMeta" })
    );
}
