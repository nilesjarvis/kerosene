use super::super::actions::HyperliquidL1Action;
use super::super::model::ExchangeOrderKind;
use super::{
    EXCHANGE_EXPIRES_AFTER_MS, EXCHANGE_REQUEST_TIMEOUT, EXCHANGE_URL, PlaceOrderRequest,
    allocate_exchange_nonce_from, build_exchange_client, build_signed_exchange_payload_with_nonce,
    exchange_payload_action, exchange_payload_contains_private_key, exchange_payload_expires_after,
    exchange_payload_nonce, exchange_payload_signature, exchange_payload_vault_address,
    exchange_request, parse_exchange_http_response, parse_exchange_response,
    redact_exchange_result,
};
use std::sync::atomic::{AtomicU64, Ordering};
use zeroize::Zeroizing;

const TEST_PRIVATE_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000001";

async fn read_test_http_request(stream: &mut tokio::net::TcpStream) -> String {
    use tokio::io::AsyncReadExt;

    let mut request = Vec::new();
    let mut chunk = [0_u8; 512];
    while request.len() < 8_192 {
        let read = stream
            .read(&mut chunk)
            .await
            .expect("test request should be readable");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8(request).expect("test HTTP request should be UTF-8")
}

#[test]
fn exchange_nonce_allocator_is_monotonic_inside_same_millisecond() {
    let last_nonce = AtomicU64::new(0);

    let first = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);
    let second = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);
    let third = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);

    assert_eq!(first, 1_700_000_000_000);
    assert_eq!(second, first + 1);
    assert_eq!(third, second + 1);
}

#[test]
fn exchange_nonce_allocator_never_moves_backwards_when_clock_regresses() {
    let last_nonce = AtomicU64::new(5_000);

    let nonce = allocate_exchange_nonce_from(&last_nonce, 4_000);

    assert_eq!(nonce, 5_001);
    assert_eq!(last_nonce.load(Ordering::SeqCst), 5_001);
}

#[test]
fn exchange_request_has_a_mutation_local_timeout() {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({"action": {"type": "cancel"}});

    let default_request = client
        .post(EXCHANGE_URL)
        .json(&payload)
        .build()
        .expect("default test request should build");
    let request = exchange_request(&client, &payload)
        .build()
        .expect("test exchange request should build");

    assert!(default_request.timeout().is_none());
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(request.url().as_str(), EXCHANGE_URL);
    assert_eq!(request.timeout(), Some(&EXCHANGE_REQUEST_TIMEOUT));
}

#[tokio::test]
async fn exchange_client_does_not_replay_a_redirected_mutation() {
    use tokio::io::AsyncWriteExt;

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("test listener should bind");
    let address = listener.local_addr().expect("listener should have address");
    let redirect_location = format!("http://{address}/replayed");
    let server = tokio::spawn(async move {
        let (mut first, _) = listener
            .accept()
            .await
            .expect("initial exchange request should arrive");
        let first_request = read_test_http_request(&mut first).await;
        assert!(first_request.starts_with("POST /exchange "));
        first
            .write_all(
                format!(
                    concat!(
                        "HTTP/1.1 307 Temporary Redirect\r\n",
                        "Location: {}\r\n",
                        "Content-Length: 0\r\n",
                        "Connection: close\r\n\r\n"
                    ),
                    redirect_location
                )
                .as_bytes(),
            )
            .await
            .expect("redirect response should write");
        drop(first);

        match tokio::time::timeout(std::time::Duration::from_millis(250), listener.accept()).await {
            Ok(Ok((mut replay, _))) => {
                let replay_request = read_test_http_request(&mut replay).await;
                replay
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .await
                    .expect("replay response should write");
                Some(replay_request)
            }
            _ => None,
        }
    });

    let client = build_exchange_client().expect("test exchange client should build");
    let response = client
        .post(format!("http://{address}/exchange"))
        .body("{}")
        .send()
        .await
        .expect("redirect response should be returned without replay");
    let replay = server.await.expect("test server should finish");

    assert_eq!(response.status(), reqwest::StatusCode::TEMPORARY_REDIRECT);
    assert!(
        replay.is_none(),
        "redirect caused a second mutation request"
    );
}

#[test]
fn signed_exchange_payload_contains_signed_request_fields_without_private_key() {
    let nonce = 1_700_000_000_000;
    let vault_address = "0x0000000000000000000000000000000000000001";
    let action = HyperliquidL1Action::cancel(110_003, 42);

    let payload = build_signed_exchange_payload_with_nonce(
        Zeroizing::new(TEST_PRIVATE_KEY.to_string()),
        &action,
        Some(vault_address),
        nonce,
    )
    .expect("payload should sign");

    assert_eq!(exchange_payload_nonce(&payload), Some(nonce));
    assert_eq!(
        exchange_payload_expires_after(&payload),
        Some(nonce + EXCHANGE_EXPIRES_AFTER_MS)
    );
    assert_eq!(
        exchange_payload_vault_address(&payload),
        Some(vault_address)
    );
    assert!(exchange_payload_action(&payload).is_some());
    let signature = exchange_payload_signature(&payload).expect("signature should be present");
    assert!(
        signature
            .get("r")
            .and_then(serde_json::Value::as_str)
            .is_some()
    );
    assert!(
        signature
            .get("s")
            .and_then(serde_json::Value::as_str)
            .is_some()
    );
    assert!(
        signature
            .get("v")
            .and_then(serde_json::Value::as_u64)
            .is_some()
    );
    assert!(!exchange_payload_contains_private_key(
        &payload,
        TEST_PRIVATE_KEY
    ));
}

#[test]
fn signed_order_payload_preserves_valid_wire_fields() {
    const CLOID: &str = "0x1234567890abcdef1234567890abcdef";
    let action = HyperliquidL1Action::order_with_cloid(
        7,
        false,
        "123.45".to_string(),
        "0.25".to_string(),
        ExchangeOrderKind::LimitIoc,
        true,
        Some(CLOID.to_string()),
    );

    let payload = build_signed_exchange_payload_with_nonce(
        Zeroizing::new(TEST_PRIVATE_KEY.to_string()),
        &action,
        None,
        1_700_000_000_000,
    )
    .expect("valid order payload should sign");

    let expected_action = serde_json::json!({
        "type": "order",
        "orders": [{
            "a": 7,
            "b": false,
            "p": "123.45",
            "s": "0.25",
            "r": true,
            "t": { "limit": { "tif": "Ioc" } },
            "c": CLOID
        }],
        "grouping": "na"
    });
    assert_eq!(exchange_payload_action(&payload), Some(&expected_action));
}

#[test]
fn signed_modify_payload_preserves_valid_wire_fields() {
    let action =
        HyperliquidL1Action::modify(42, 7, false, "123.45".to_string(), "0.25".to_string(), true);

    let payload = build_signed_exchange_payload_with_nonce(
        Zeroizing::new(TEST_PRIVATE_KEY.to_string()),
        &action,
        None,
        1_700_000_000_000,
    )
    .expect("valid modify payload should sign");

    let expected_action = serde_json::json!({
        "type": "batchModify",
        "modifies": [{
            "oid": 42,
            "order": {
                "a": 7,
                "b": false,
                "p": "123.45",
                "s": "0.25",
                "r": true,
                "t": { "limit": { "tif": "Gtc" } }
            }
        }]
    });
    assert_eq!(exchange_payload_action(&payload), Some(&expected_action));
}

#[test]
fn signed_order_payload_rejects_invalid_wire_numbers_before_signing() {
    let invalid_prices = ["", "NaN", "inf", "-inf", "0", "-0", "-1", "wire-secret"];
    for invalid_price in invalid_prices {
        let action = HyperliquidL1Action::order_with_cloid(
            7,
            true,
            invalid_price.to_string(),
            "1".to_string(),
            ExchangeOrderKind::Limit,
            false,
            Some("0x1234567890abcdef1234567890abcdef".to_string()),
        );

        let error = build_signed_exchange_payload_with_nonce(
            Zeroizing::new(TEST_PRIVATE_KEY.to_string()),
            &action,
            None,
            1_700_000_000_000,
        )
        .expect_err("invalid wire price must fail before signing");

        assert_eq!(
            error,
            "Order action blocked: wire price must be a positive finite number"
        );
        assert!(!error.contains("wire-secret"));
    }

    for invalid_size in ["NaN", "inf", "0", "-0", "-1", "size-secret"] {
        let action = HyperliquidL1Action::order_with_cloid(
            7,
            true,
            "100".to_string(),
            invalid_size.to_string(),
            ExchangeOrderKind::Limit,
            false,
            Some("0x1234567890abcdef1234567890abcdef".to_string()),
        );

        let error = build_signed_exchange_payload_with_nonce(
            Zeroizing::new(TEST_PRIVATE_KEY.to_string()),
            &action,
            None,
            1_700_000_000_000,
        )
        .expect_err("invalid wire size must fail before signing");

        assert_eq!(
            error,
            "Order action blocked: wire size must be a positive finite number"
        );
        assert!(!error.contains("size-secret"));
    }
}

#[test]
fn signed_modify_payload_rejects_invalid_wire_numbers_before_signing() {
    let action =
        HyperliquidL1Action::modify(42, 7, false, "100".to_string(), "NaN".to_string(), true);

    let error = build_signed_exchange_payload_with_nonce(
        Zeroizing::new(TEST_PRIVATE_KEY.to_string()),
        &action,
        None,
        1_700_000_000_000,
    )
    .expect_err("invalid modify size must fail before signing");

    assert_eq!(
        error,
        "Order action blocked: wire size must be a positive finite number"
    );
}

#[test]
fn signed_order_payload_requires_a_128_bit_hex_cloid() {
    for cloid in [
        None,
        Some("0x1234".to_string()),
        Some("0x1234567890abcdef1234567890abcdeg".to_string()),
        Some("1234567890abcdef1234567890abcdef".to_string()),
    ] {
        let action = HyperliquidL1Action::order_with_cloid(
            7,
            true,
            "100".to_string(),
            "1".to_string(),
            ExchangeOrderKind::Limit,
            false,
            cloid,
        );

        let error = build_signed_exchange_payload_with_nonce(
            Zeroizing::new(TEST_PRIVATE_KEY.to_string()),
            &action,
            None,
            1_700_000_000_000,
        )
        .expect_err("missing or malformed CLOID must fail before signing");

        assert_eq!(
            error,
            "Order action blocked: client order ID must be 128-bit hexadecimal"
        );
        assert!(!error.contains("1234567890abcdef"));
    }
}

#[test]
fn signed_exchange_payload_error_does_not_echo_private_key() {
    let invalid_key = format!("{TEST_PRIVATE_KEY}ff");
    let action = HyperliquidL1Action::cancel(110_003, 42);

    let error = build_signed_exchange_payload_with_nonce(
        Zeroizing::new(invalid_key.clone()),
        &action,
        None,
        1_700_000_000_000,
    )
    .expect_err("invalid private key should fail before posting");

    assert!(error.contains("Invalid private key hex"));
    assert!(!error.contains(&invalid_key));
    assert!(!error.contains(TEST_PRIVATE_KEY));
}

#[test]
fn place_order_request_debug_redacts_order_values_and_cloid() {
    let request = PlaceOrderRequest {
        asset: 110_003,
        is_buy: true,
        price: "price-secret".to_string(),
        size: "size-secret".to_string(),
        order_kind: ExchangeOrderKind::Limit,
        reduce_only: true,
        cloid: Some("0x11111111111111111111111111111111".to_string()),
    };

    let rendered = format!("{request:?}");

    assert!(rendered.contains("PlaceOrderRequest"));
    assert!(rendered.contains("asset: 110003"));
    assert!(rendered.contains("is_buy: true"));
    assert!(rendered.contains("order_kind: Limit"));
    assert!(rendered.contains("reduce_only: true"));
    assert!(rendered.contains("has_cloid: true"));
    assert!(rendered.contains("<redacted>"));
    for secret in [
        "price-secret",
        "size-secret",
        "0x11111111111111111111111111111111",
    ] {
        assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
    }
}

#[test]
fn parse_exchange_response_accepts_valid_exchange_json() {
    let response = parse_exchange_response(r#"{"status":"ok","response":{"type":"order"}}"#)
        .expect("valid exchange response should parse");

    assert_eq!(response.status, "ok");
    assert_eq!(
        response
            .response
            .as_ref()
            .expect("response body should be parsed")
            .response_type,
        "order"
    );
}

#[test]
fn non_success_http_status_cannot_confirm_a_successful_mutation() {
    let raw = r#"{"status":"ok","response":{"type":"order","data":{"statuses":[{"resting":{"oid":42}}]}}}"#;

    let success = parse_exchange_http_response(reqwest::StatusCode::OK, raw)
        .expect("success HTTP envelope should preserve response");
    assert_eq!(success.order_oid(), Some(42));

    let error = parse_exchange_http_response(reqwest::StatusCode::INTERNAL_SERVER_ERROR, raw)
        .expect_err("non-success HTTP envelope must not confirm success");

    assert_eq!(
        error,
        "Exchange response status uncertain: HTTP 500 Internal Server Error"
    );
    assert!(!error.contains("42"));
}

#[test]
fn non_success_http_status_preserves_structured_rejection_and_conflict() {
    let rejection = parse_exchange_http_response(
        reqwest::StatusCode::BAD_REQUEST,
        r#"{"status":"ok","response":{"type":"order","data":{"statuses":[{"error":"Order rejected"}]}}}"#,
    )
    .expect("structured rejection should remain classifiable");
    assert!(rejection.is_error());
    assert!(!rejection.has_potential_order_effect());

    let conflict = parse_exchange_http_response(
        reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        r#"{"status":"ok","response":{"type":"order","data":{"statuses":[{"resting":{"oid":42}},{"error":"conflicting rejection"}]}}}"#,
    )
    .expect("structured conflict should reach ambiguity classification");
    assert!(conflict.has_conflicting_order_effect());
    assert!(conflict.is_ambiguous_order_result());
}

#[test]
fn parse_exchange_response_redacts_sensitive_raw_body_on_error() {
    let signature = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let cloid = "0x1234567890abcdef1234567890abcdef";
    let raw = format!(
        "upstream parse failed Authorization: Bearer exchange-secret api_key=\"json-secret\" txid={signature} cloid={cloid}"
    );

    let error = parse_exchange_response(&raw).expect_err("malformed body should fail");

    assert!(error.contains("Exchange error:"));
    assert!(error.contains("<redacted>"));
    assert!(error.contains("<redacted-hex>"));
    for secret in [
        "exchange-secret",
        "json-secret",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        cloid,
    ] {
        assert!(
            !error.contains(secret),
            "exchange parse error leaked {secret}"
        );
    }
}

#[test]
fn exchange_result_error_is_redacted_before_message_mapping() {
    let private_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let cloid = "0x1234567890abcdef1234567890abcdef";
    let raw = format!(
        concat!(
            "Exchange request failed: Authorization: Bearer bearer-secret ",
            "api_key=api-secret private_key={} trace=0x{} cloid={}"
        ),
        private_key, private_key, cloid
    );

    let result = redact_exchange_result(Err(raw));
    let debug = format!("{result:?}");
    let error = result.expect_err("error should remain an error");

    assert!(error.contains("Exchange request failed"));
    assert!(error.contains("<redacted>"));
    assert!(error.contains("<redacted-hex>"));
    for secret in ["bearer-secret", "api-secret", private_key, cloid] {
        assert!(!error.contains(secret), "exchange result leaked {secret}");
        assert!(!debug.contains(secret), "result debug leaked {secret}");
    }
}

#[test]
fn exchange_result_redaction_preserves_success_and_safe_error_text() {
    let response = parse_exchange_response(r#"{"status":"ok","response":{"type":"default"}}"#)
        .expect("valid response should parse");
    let response = redact_exchange_result(Ok(response)).expect("success should remain successful");
    assert_eq!(response.status, "ok");
    assert_eq!(response.summary(), "OK (default)");

    let safe = "Exchange request failed: connection closed before response";

    let error = redact_exchange_result(Err(safe.to_string()))
        .expect_err("safe error should remain an error");

    assert_eq!(error, safe);
}
