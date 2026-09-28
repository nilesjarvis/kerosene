use super::*;

#[test]
fn tool_runtime_debug_omits_request_detail() {
    let event = AgentRuntimeEvent::ToolStarted {
        generation: 1,
        call_id: "call-1".to_string(),
        name: "kerosene_data".to_string(),
        detail: Some("private account context".to_string()),
    };
    let debug = format!("{event:?}");
    assert!(!debug.contains("private account context"));
    assert!(debug.contains("kerosene_data"));
}

#[test]
fn runtime_event_debug_redacts_model_text() {
    let event = AgentRuntimeEvent::TextDelta {
        generation: 1,
        delta: "private portfolio answer".to_string(),
        total_tokens: None,
        total_cost_usd: None,
    };
    let debug = format!("{event:?}");
    assert!(!debug.contains("private portfolio answer"));
    assert!(debug.contains("<redacted>"));
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
