use serde_json::Value;

// ---------------------------------------------------------------------------
// User Stream Routing
// ---------------------------------------------------------------------------

pub(super) fn normalize_ws_user_address(input: &str) -> Option<String> {
    let address = input.trim().to_lowercase();
    let hex = address.strip_prefix("0x")?;
    (hex.len() == 40 && hex.chars().all(|c| c.is_ascii_hexdigit())).then_some(address)
}

pub(super) fn matching_user_payload_address(
    data: &Value,
    expected_user: Option<&str>,
) -> Option<String> {
    let expected = expected_user.and_then(normalize_ws_user_address)?;
    let actual = data
        .get("user")
        .and_then(|value| value.as_str())
        .and_then(normalize_ws_user_address)?;
    (actual == expected).then_some(actual)
}

#[cfg(test)]
mod ws_user_routing_tests {
    use super::*;

    #[test]
    fn user_payload_address_must_match_stream_target() {
        let target = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let other = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let payload = serde_json::json!({
            "user": target.to_uppercase(),
            "orders": []
        });

        assert_eq!(
            matching_user_payload_address(&payload, Some(target)).as_deref(),
            Some(target)
        );
        assert!(matching_user_payload_address(&payload, Some(other)).is_none());
        assert!(matching_user_payload_address(&payload, None).is_none());
    }

    #[test]
    fn user_payload_address_is_required_for_private_routing() {
        let target = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let payload = serde_json::json!({
            "orders": []
        });

        assert!(matching_user_payload_address(&payload, Some(target)).is_none());
    }
}
