use super::*;
use serde_json::json;

#[test]
fn provider_labels_preserve_known_names_and_unknown_id_prefixes() {
    for (id, expected) in [
        ("openai/model", "OpenAI"),
        ("anthropic/model", "Anthropic"),
        ("google/model", "Google"),
        ("meta-llama/model", "Meta"),
        ("mistralai/model", "Mistral AI"),
        ("x-ai/model", "xAI"),
        ("deepseek/model", "DeepSeek"),
        ("openrouter/auto", "OpenRouter"),
        ("", "Unknown provider"),
        ("/model", "Unknown provider"),
        ("OpenAI/model", "OpenAI"),
        ("vendor/model/variant", "vendor"),
        ("vendor", "vendor"),
        ("供应商/模型", "供应商"),
        (" spaced /model", " spaced "),
    ] {
        let mut model = OpenRouterModel::auto_router();
        model.id = id.to_string();
        assert_eq!(model.provider_summary(), expected);
    }
}

#[test]
fn catalog_limits_apply_before_filtering_and_first_trimmed_id_wins() {
    let mut data = vec![json!({"id": "not-a-tool"}); 1_000];
    data.push(json!({"id": "beyond-limit", "supported_parameters": ["tools"]}));
    let models =
        parse_model_catalog_response(&json!({"data": data}).to_string()).expect("bounded catalog");
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, DEFAULT_OPENROUTER_MODEL);

    let long_name = "界".repeat(161);
    let data = json!({"data": [
        {"id": " vendor/model ", "name": long_name, "context_length": 0,
            "supported_parameters": ["tools"]},
        {"id": "vendor/model", "name": "ignored duplicate", "context_length": 123,
            "supported_parameters": ["tools"]},
        {"id": " 名称 ", "name": " \t", "supported_parameters": ["tools"]}
    ]});
    let models = parse_model_catalog_response(&data.to_string()).expect("trimmed catalog");
    assert_eq!(models.len(), 3);
    assert_eq!(models[1].id, "vendor/model");
    assert_eq!(models[1].name, format!("{}…", "界".repeat(159)));
    assert_eq!(models[1].context_length, None);
    assert_eq!(models[2].id, "名称");
    assert_eq!(models[2].name, "名称");
}

#[test]
fn model_catalog_keeps_tool_models_and_normalizes_openrouter_pricing() {
    let text = r#"{
        "data": [
            {
                "id": "openai/gpt-tool",
                "name": "OpenAI: GPT Tool",
                "context_length": 128000,
                "pricing": {
                    "prompt": "0.0000025",
                    "completion": "0.00001",
                    "internal_reasoning": "0.000005",
                    "request": "0.01",
                    "overrides": [{"min_prompt_tokens": 100000, "prompt": "0.000005"}]
                },
                "architecture": {"input_modalities": ["text", "image"]},
                "supported_parameters": ["temperature", "tools"]
            },
            {
                "id": "example/no-tools",
                "name": "No tools",
                "context_length": 32000,
                "pricing": {"prompt": "0", "completion": "0"},
                "supported_parameters": ["temperature"]
            },
            {
                "id": "openrouter/auto",
                "name": "API auto entry",
                "supported_parameters": ["tools"]
            }
        ]
    }"#;

    let models = parse_model_catalog_response(text).expect("catalog should parse");

    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, DEFAULT_OPENROUTER_MODEL);
    assert_eq!(
        models[0].pricing_summary(),
        "Pricing varies by routed model"
    );
    assert_eq!(models[1].id, "openai/gpt-tool");
    assert_eq!(models[1].context_length, Some(128_000));
    assert_eq!(models[1].prompt_price_per_million_usd, Some(2.5));
    assert_eq!(models[1].completion_price_per_million_usd, Some(10.0));
    assert_eq!(models[1].reasoning_price_per_million_usd, Some(5.0));
    assert_eq!(models[1].request_price_usd, Some(0.01));
    assert!(models[1].has_conditional_pricing);
    assert!(models[0].supports_image_input);
    assert!(models[1].supports_image_input);
    assert_eq!(models[1].provider_summary(), "OpenAI");
    assert_eq!(
        models[1].pricing_summary(),
        "$2.50/M input · $10.00/M output · $5.00/M reasoning · $0.010/request · variable rates"
    );
    assert_eq!(models[1].context_summary(), "128K context");
}

#[test]
fn model_catalog_ignores_duplicate_ids_and_invalid_prices() {
    let text = r#"{
        "data": [
            {
                "id": "vendor/model",
                "name": "Vendor Model",
                "pricing": {"prompt": "not-a-price", "completion": "-1"},
                "supported_parameters": ["tools"]
            },
            {
                "id": "vendor/model",
                "name": "Duplicate",
                "pricing": {"prompt": "1", "completion": "1"},
                "supported_parameters": ["tools"]
            }
        ]
    }"#;

    let models = parse_model_catalog_response(text).expect("catalog should parse");

    assert_eq!(models.len(), 2);
    assert_eq!(models[1].name, "Vendor Model");
    assert_eq!(models[1].prompt_price_per_million_usd, None);
    assert_eq!(models[1].completion_price_per_million_usd, None);
    assert!(!models[1].supports_image_input);
    assert_eq!(models[1].pricing_summary(), "Pricing unavailable");
}

#[test]
fn model_catalog_requires_the_documented_data_envelope() {
    let error =
        parse_model_catalog_response(r#"{"models": []}"#).expect_err("missing data should fail");

    assert!(error.contains("model catalog parse failed"));
}

#[test]
fn model_catalog_fetch_rejects_missing_key_before_any_io() {
    let error = futures::executor::block_on(fetch_tool_models(Zeroizing::new(String::new())))
        .expect_err("missing key should fail");

    assert!(error.contains("OpenRouter API key is required"));
}
