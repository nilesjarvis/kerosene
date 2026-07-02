use super::*;

// ---------------------------------------------------------------------------
// Request Construction
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Chat Completion Response Parsing
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Key Status Parsing
// ---------------------------------------------------------------------------

#[test]
fn key_status_response_parses_full_payload() {
    let text = r#"{
        "data": {
            "label": "kerosene",
            "usage": 25.5,
            "limit": 100.0,
            "limit_remaining": 74.5,
            "is_free_tier": false,
            "rate_limit": {"requests": 10, "interval": "10s"}
        }
    }"#;

    let status = parse_key_status_response(text).expect("full key status should parse");

    assert_eq!(
        status,
        OpenRouterKeyStatus {
            usage_usd: 25.5,
            limit_usd: Some(100.0),
            limit_remaining_usd: Some(74.5),
            is_free_tier: false,
        }
    );
}

#[test]
fn key_status_response_defaults_missing_fields() {
    let status =
        parse_key_status_response(r#"{"data": {}}"#).expect("minimal key status should parse");

    assert_eq!(
        status,
        OpenRouterKeyStatus {
            usage_usd: 0.0,
            limit_usd: None,
            limit_remaining_usd: None,
            is_free_tier: false,
        }
    );
}

#[test]
fn key_status_response_without_data_is_an_error() {
    let error =
        parse_key_status_response(r#"{"unexpected": true}"#).expect_err("missing data should fail");

    assert!(error.contains("key check parse failed"));
}

// ---------------------------------------------------------------------------
// HTTP Error Mapping
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Request Guards
// ---------------------------------------------------------------------------

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

#[test]
fn key_status_fetch_rejects_missing_key_before_any_io() {
    let error = futures::executor::block_on(fetch_key_status(Zeroizing::new(String::new())))
        .expect_err("missing key should fail");

    assert!(error.contains("OpenRouter API key is required"));
}
