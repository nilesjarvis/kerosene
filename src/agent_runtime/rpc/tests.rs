use super::*;

#[test]
fn parses_text_delta_and_usage() {
    let value = json!({
        "type": "message_update",
        "usage": { "totalTokens": 42, "cost": { "total": 0.001 } },
        "assistantMessageEvent": { "type": "text_delta", "delta": "secret reply" }
    });

    assert!(matches!(
        parse_rpc_event(7, &value),
        Some(AgentRuntimeEvent::TextDelta {
            generation: 7,
            delta,
            total_tokens: Some(42),
            total_cost_usd: Some(cost),
        }) if delta == "secret reply" && (cost - 0.001).abs() < f64::EPSILON
    ));
}

#[test]
fn parses_reasoning_lifecycle_without_exposing_it_in_debug() {
    let started = json!({
        "type": "message_update",
        "assistantMessageEvent": { "type": "thinking_start", "contentIndex": 0 }
    });
    let delta = json!({
        "type": "message_update",
        "assistantMessageEvent": {
            "type": "thinking_delta",
            "contentIndex": 0,
            "delta": "private portfolio reasoning"
        }
    });
    let finished = json!({
        "type": "message_update",
        "assistantMessageEvent": { "type": "thinking_end", "contentIndex": 0 }
    });

    assert!(matches!(
        parse_rpc_event(7, &started),
        Some(AgentRuntimeEvent::ReasoningStarted { generation: 7 })
    ));
    let Some(event) = parse_rpc_event(7, &delta) else {
        panic!("expected reasoning delta");
    };
    let AgentRuntimeEvent::ReasoningDelta {
        generation: 7,
        delta: reasoning,
    } = &event
    else {
        panic!("expected reasoning delta");
    };
    assert_eq!(reasoning, "private portfolio reasoning");
    assert!(!format!("{event:?}").contains("private portfolio reasoning"));
    assert!(matches!(
        parse_rpc_event(7, &finished),
        Some(AgentRuntimeEvent::ReasoningFinished { generation: 7 })
    ));
}

#[test]
fn parses_nested_message_usage_from_current_pi_events() {
    let value = json!({
        "type": "message_update",
        "assistantMessageEvent": { "type": "text_delta", "delta": "reply" },
        "message": {
            "usage": { "totalTokens": 84, "cost": { "total": 0.002 } }
        }
    });

    assert!(matches!(
        parse_rpc_event(8, &value),
        Some(AgentRuntimeEvent::TextDelta {
            generation: 8,
            total_tokens: Some(84),
            total_cost_usd: Some(cost),
            ..
        }) if (cost - 0.002).abs() < f64::EPSILON
    ));
}

#[test]
fn parses_tool_call_with_human_readable_request_detail() {
    let value = json!({
        "type": "tool_execution_start",
        "toolCallId": "call-1",
        "toolName": "kerosene_ohlcv",
        "args": {
            "symbol": "BTC",
            "interval": "1h",
            "limit": 200,
            "private_key": "must-not-be-rendered"
        }
    });

    assert!(matches!(
        parse_rpc_event(12, &value),
        Some(AgentRuntimeEvent::ToolStarted {
            generation: 12,
            call_id,
            name,
            detail: Some(detail),
        }) if call_id == "call-1"
            && name == "kerosene_ohlcv"
            && detail == "BTC · 1h candles · Up to 200 rows"
            && !detail.contains("must-not-be-rendered")
    ));
}

#[test]
fn parses_and_redacts_extension_ui_host_action_requests() {
    let value = json!({
        "type": "extension_ui_request",
        "id": "rpc-1",
        "method": "input",
        "title": "KEROSENE_HOST_ACTION_V1",
        "placeholder": "{\"private\":\"workspace request\"}",
    });

    let event = parse_rpc_event(12, &value).expect("extension UI event");
    assert!(matches!(
        &event,
        AgentRuntimeEvent::ExtensionUiRequest {
            generation: 12,
            request_id,
            method,
            title: Some(title),
            payload: Some(payload),
        } if request_id == "rpc-1"
            && method == "input"
            && title == "KEROSENE_HOST_ACTION_V1"
            && payload.contains("workspace request")
    ));
    assert!(!format!("{event:?}").contains("workspace request"));
}

#[test]
fn extension_ui_response_preserves_rpc_correlation() {
    let accepted =
        extension_ui_response_value("rpc-1".to_string(), Some("{\"success\":true}".to_string()));
    let cancelled = extension_ui_response_value("rpc-2".to_string(), None);

    assert_eq!(accepted["type"], "extension_ui_response");
    assert_eq!(accepted["id"], "rpc-1");
    assert_eq!(accepted["value"], "{\"success\":true}");
    assert_eq!(cancelled["id"], "rpc-2");
    assert_eq!(cancelled["cancelled"], true);
}

#[test]
fn current_pi_agent_end_settles_and_aggregates_session_usage() {
    let value = json!({
        "type": "agent_end",
        "willRetry": false,
        "messages": [
            { "role": "user", "content": [] },
            {
                "role": "assistant",
                "usage": { "totalTokens": 50, "cost": { "total": 0.001 } }
            },
            {
                "role": "assistant",
                "usage": { "totalTokens": 25, "cost": { "total": 0.0005 } }
            }
        ]
    });

    assert!(matches!(
        parse_rpc_event(9, &value),
        Some(AgentRuntimeEvent::Settled {
            generation: 9,
            total_tokens: Some(75),
            total_cost_usd: Some(cost),
            has_visible_text: Some(false),
        }) if (cost - 0.0015).abs() < f64::EPSILON
    ));
}

#[test]
fn retrying_agent_end_does_not_mark_the_turn_settled() {
    let value = json!({
        "type": "agent_end",
        "willRetry": true,
        "messages": []
    });

    assert!(parse_rpc_event(10, &value).is_none());
}

#[test]
fn current_pi_agent_end_reports_visible_answer_text() {
    let value = json!({
        "type": "agent_end",
        "willRetry": false,
        "messages": [
            {
                "role": "assistant",
                "content": [
                    { "type": "thinking", "thinking": "hidden" },
                    { "type": "text", "text": "## Visible answer" }
                ]
            }
        ]
    });

    assert!(matches!(
        parse_rpc_event(11, &value),
        Some(AgentRuntimeEvent::Settled {
            generation: 11,
            has_visible_text: Some(true),
            ..
        })
    ));
}

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
