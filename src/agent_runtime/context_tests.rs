use super::*;

#[test]
fn parses_model_and_available_context_from_rpc_state() {
    let value = json!({
        "type": "response",
        "command": "get_state",
        "success": true,
        "data": {
            "model": {
                "provider": "openrouter",
                "id": "anthropic/claude-sonnet-4.5",
                "contextWindow": 1_000_000
            }
        }
    });

    assert!(matches!(
        parse_rpc_event(7, &value),
        Some(AgentRuntimeEvent::ModelContext {
            generation: 7,
            model: Some(model),
            context_window: Some(1_000_000),
        }) if model == "anthropic/claude-sonnet-4.5"
    ));
}

#[test]
fn parses_used_context_from_session_stats() {
    let value = json!({
        "type": "response",
        "command": "get_session_stats",
        "success": true,
        "data": {
            "contextUsage": {
                "tokens": 12_345,
                "contextWindow": 200_000,
                "percent": 6.1725
            }
        }
    });

    assert!(matches!(
        parse_rpc_event(8, &value),
        Some(AgentRuntimeEvent::ContextUsage {
            generation: 8,
            context_tokens: Some(12_345),
            context_window: Some(200_000),
        })
    ));
}

#[test]
fn preserves_unknown_usage_after_compaction_and_ignores_inspection_errors() {
    let compacted = json!({
        "type": "response",
        "command": "get_session_stats",
        "success": true,
        "data": {
            "contextUsage": {
                "tokens": null,
                "contextWindow": 200_000,
                "percent": null
            }
        }
    });
    assert!(matches!(
        parse_rpc_event(9, &compacted),
        Some(AgentRuntimeEvent::ContextUsage {
            context_tokens: None,
            context_window: Some(200_000),
            ..
        })
    ));

    let failed = json!({
        "type": "response",
        "command": "get_session_stats",
        "success": false,
        "error": "metrics unavailable"
    });
    assert!(parse_rpc_event(9, &failed).is_none());
}

#[test]
fn runtime_event_debug_redacts_resolved_model_text() {
    let event = AgentRuntimeEvent::ModelContext {
        generation: 10,
        model: Some("private-model-alias".to_string()),
        context_window: Some(200_000),
    };
    let debug = format!("{event:?}");

    assert!(!debug.contains("private-model-alias"));
    assert!(debug.contains("<redacted>"));
}
