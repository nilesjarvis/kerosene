use super::*;
use serde_json::json;

#[tokio::test]
async fn shared_transport_preserves_body_and_distinguishes_failure_stages() {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("fixture client");
    for context in ["chat completion", "model catalog", "key check"] {
        for (status, body, truncated) in [
            (200, "  Σ → 🦀\n ", false),
            (
                429,
                r#"{"error":{"message":"limit token=test-secret"}}"#,
                false,
            ),
            (500, "api_key=test-secret", false),
            (200, "partial", true),
            (429, "partial", true),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("fixture listener");
            let address = listener.local_addr().expect("fixture address");
            let server = tokio::spawn(async move {
                tokio::time::timeout(Duration::from_secs(3), async move {
                    let (mut stream, _) = listener.accept().await.expect("fixture connection");
                    let mut request = Vec::new();
                    while !request.windows(7).any(|part| part == b"PAYLOAD") {
                        let mut buffer = [0; 1_024];
                        let read = stream.read(&mut buffer).await.expect("fixture request");
                        assert!(read > 0 && request.len() < 8_192);
                        request.extend_from_slice(&buffer[..read]);
                    }
                    assert!(request.starts_with(b"POST /fixture HTTP/1.1\r\n"));
                    let request = String::from_utf8(request).expect("ASCII request");
                    assert!(request.contains("x-fixture: preserved\r\n"));
                    let length = body.len() + usize::from(truncated);
                    let response = format!(
                        "HTTP/1.1 {status} Fixture\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}"
                    );
                    stream.write_all(response.as_bytes()).await.expect("fixture response");
                })
                .await
                .expect("fixture deadline");
            });
            let result = send_request(
                client
                    .post(format!("http://{address}/fixture"))
                    .header("x-fixture", "preserved")
                    .body("PAYLOAD"),
                context,
            )
            .await;
            server.await.expect("fixture task");

            if truncated {
                assert!(
                    result
                        .expect_err("truncated body")
                        .starts_with(&format!("OpenRouter {context} response read failed:"))
                );
            } else if status == 200 {
                assert_eq!(result.expect("successful response"), body);
            } else {
                let expected = if status == 429 {
                    format!("OpenRouter {context} HTTP 429 (rate limited): limit token=<redacted>")
                } else {
                    format!("OpenRouter {context} HTTP 500: api_key=<redacted>")
                };
                assert_eq!(result.expect_err("HTTP error"), expected);
            }
        }
        let error = send_request(client.get("not a URL"), context)
            .await
            .expect_err("invalid request builder");
        assert!(error.starts_with(&format!("OpenRouter {context} request failed:")));
    }
}

#[test]
fn content_parts_preserve_unicode_whitespace_and_first_choice_precedence() {
    for (parts, expected) in [
        (json!([]), None),
        (json!([{}, {"text": ""}]), None),
        (json!([{"text": " \n"}, {"text": "\t"}]), None),
        (
            json!([{"text": " 前 "}, {}, {"text": "e\u{301} → 🦀\n"}]),
            Some(" 前 e\u{301} → 🦀\n"),
        ),
        (
            json!([{"text": ""}, {"text": "a"}, {"text": "b"}, {"text": " "}]),
            Some("ab "),
        ),
    ] {
        let body = json!({"choices": [
            {"message": {"content": parts}, "finish_reason": "first"},
            {"message": {"content": "ignored second choice"}}
        ]});
        let result = parse_chat_completion_response(&body.to_string());
        if let Some(expected) = expected {
            let result = result.expect("nonempty first choice");
            assert_eq!(result.content, expected);
            assert_eq!(result.finish_reason.as_deref(), Some("first"));
        } else {
            assert_eq!(
                result.expect_err("empty first choice"),
                "OpenRouter chat completion returned no content"
            );
        }
    }
    let body = json!({
        "choices": [{"message": {"content": [{"text": "otherwise valid"}]}}],
        "error": {"message": "upstream failure"}
    });
    assert_eq!(
        parse_chat_completion_response(&body.to_string()).expect_err("error overrides content"),
        "OpenRouter chat completion failed: upstream failure"
    );
}

#[test]
fn chat_completion_request_serializes_model_roles_and_content() {
    let request = ChatCompletionRequest::new(
        "openrouter/auto",
        vec![
            ChatMessage::system("Summarize filings."),
            ChatMessage::user("10-K excerpt"),
        ],
    );

    let json = serde_json::to_value(&request).expect("request should serialize");

    assert_eq!(json["model"], "openrouter/auto");
    assert_eq!(json["messages"][0]["role"], "system");
    assert_eq!(json["messages"][0]["content"], "Summarize filings.");
    assert_eq!(json["messages"][1]["role"], "user");
    assert_eq!(json["messages"][1]["content"], "10-K excerpt");
}

#[test]
fn chat_completion_request_omits_unset_optional_fields() {
    let request = ChatCompletionRequest::new("openrouter/auto", vec![ChatMessage::user("hi")]);

    let json = serde_json::to_value(&request).expect("request should serialize");
    let object = json.as_object().expect("request should be an object");

    assert!(!object.contains_key("max_tokens"));
    assert!(!object.contains_key("temperature"));
    assert!(!object.contains_key("stream"));
}

#[test]
fn chat_completion_request_serializes_set_optional_fields() {
    let mut request = ChatCompletionRequest::new("openrouter/auto", vec![ChatMessage::user("hi")]);
    request.max_tokens = Some(512);
    request.temperature = Some(0.25);

    let json = serde_json::to_value(&request).expect("request should serialize");

    assert_eq!(json["max_tokens"], 512);
    assert_eq!(json["temperature"], 0.25);
}

#[test]
fn chat_completion_response_parses_text_content_and_usage() {
    let text = r#"{
        "id": "gen-1",
        "model": "openai/gpt-4o-mini",
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": "A concise summary."},
                "finish_reason": "stop"
            }
        ],
        "usage": {"prompt_tokens": 120, "completion_tokens": 40, "total_tokens": 160}
    }"#;

    let completion = parse_chat_completion_response(text).expect("text completion should parse");

    assert_eq!(completion.model, "openai/gpt-4o-mini");
    assert_eq!(completion.content, "A concise summary.");
    assert_eq!(completion.finish_reason.as_deref(), Some("stop"));
    assert_eq!(
        completion.usage,
        Some(TokenUsage {
            prompt_tokens: 120,
            completion_tokens: 40,
            total_tokens: 160,
        })
    );
}

#[test]
fn chat_completion_response_parses_content_parts() {
    let text = r#"{
        "model": "openai/gpt-4o-mini",
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": [
                        {"type": "text", "text": "Part one. "},
                        {"type": "text", "text": "Part two."}
                    ]
                },
                "finish_reason": "stop"
            }
        ]
    }"#;

    let completion =
        parse_chat_completion_response(text).expect("part-based completion should parse");

    assert_eq!(completion.content, "Part one. Part two.");
    assert_eq!(completion.usage, None);
}

#[test]
fn chat_completion_response_without_choices_is_an_error() {
    let error = parse_chat_completion_response(r#"{"model": "openai/gpt-4o-mini", "choices": []}"#)
        .expect_err("empty choices should fail");

    assert!(error.contains("no choices"));
}

#[test]
fn chat_completion_response_without_content_is_an_error() {
    let text = r#"{
        "choices": [{"message": {"role": "assistant", "content": null}, "finish_reason": "stop"}]
    }"#;

    let error = parse_chat_completion_response(text).expect_err("missing content should fail");

    assert!(error.contains("no content"));
}

#[test]
fn chat_completion_response_with_top_level_error_is_an_error() {
    let text = r#"{
        "error": {"code": 502, "message": "Provider returned error token=provider-secret"},
        "choices": []
    }"#;

    let error = parse_chat_completion_response(text).expect_err("embedded error should fail");

    assert!(error.contains("OpenRouter chat completion failed"));
    assert!(error.contains("Provider returned error"));
    assert!(error.contains("token=<redacted>"));
    assert!(!error.contains("provider-secret"));
}

#[test]
fn chat_completion_response_with_invalid_json_is_an_error() {
    let error = parse_chat_completion_response("not json").expect_err("invalid JSON should fail");

    assert!(error.contains("parse failed"));
}

#[test]
fn http_error_uses_error_envelope_message_and_status_hint() {
    let rendered = openrouter_http_error(
        "chat completion",
        401,
        r#"{"error": {"code": 401, "message": "No auth credentials found"}}"#,
    );

    assert!(rendered.contains("HTTP 401"));
    assert!(rendered.contains("invalid or disabled API key"));
    assert!(rendered.contains("No auth credentials found"));
}

#[test]
fn http_error_maps_payment_and_rate_limit_hints() {
    assert!(
        openrouter_http_error("chat completion", 402, "{}")
            .contains("insufficient OpenRouter credits")
    );
    assert!(openrouter_http_error("chat completion", 429, "{}").contains("rate limited"));
    assert!(
        openrouter_http_error("chat completion", 503, "{}")
            .contains("model or provider unavailable")
    );
}

#[test]
fn http_error_falls_back_to_redacted_body_snippet() {
    let rendered = openrouter_http_error("key check", 500, "upstream blew up api_key=oops-secret");

    assert!(rendered.contains("HTTP 500"));
    assert!(rendered.contains("upstream blew up"));
    assert!(rendered.contains("api_key=<redacted>"));
    assert!(!rendered.contains("oops-secret"));
}

#[test]
fn chat_completion_rejects_missing_key_model_and_messages_before_any_io() {
    let request = ChatCompletionRequest::new("openrouter/auto", vec![ChatMessage::user("hi")]);
    let error =
        futures::executor::block_on(chat_completion(request, Zeroizing::new(String::new())))
            .expect_err("missing key should fail");
    assert!(error.contains("OpenRouter API key is required"));

    let request = ChatCompletionRequest::new("  ", vec![ChatMessage::user("hi")]);
    let error = futures::executor::block_on(chat_completion(
        request,
        Zeroizing::new("sk-or-test".to_string()),
    ))
    .expect_err("missing model should fail");
    assert!(error.contains("missing model"));

    let request = ChatCompletionRequest::new("openrouter/auto", Vec::new());
    let error = futures::executor::block_on(chat_completion(
        request,
        Zeroizing::new("sk-or-test".to_string()),
    ))
    .expect_err("missing messages should fail");
    assert!(error.contains("missing messages"));
}
