use super::*;

#[test]
fn http_auth_errors_show_key_guidance_without_body_snippet() {
    let message = hyperdash_http_error(
        "heatmap",
        StatusCode::UNAUTHORIZED,
        "provider response that should not be surfaced",
    );

    assert!(message.contains("authentication failed"));
    assert!(message.contains("Settings > Integrations"));
    assert!(!message.contains("provider response"));
}

#[test]
fn graphql_auth_errors_show_key_guidance() {
    let message =
        hyperdash_graphql_error("liquidation levels", vec!["Invalid API key".to_string()]);

    assert!(message.contains("authentication failed"));
    assert!(message.contains("Settings > Integrations"));
}

#[test]
fn non_auth_http_errors_keep_status_and_response_context() {
    let message = hyperdash_http_error("heatmap", StatusCode::BAD_GATEWAY, "upstream unavailable");

    assert!(message.contains("HTTP 502"));
    assert!(message.contains("upstream unavailable"));
}
