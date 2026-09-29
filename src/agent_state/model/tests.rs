use super::*;

#[test]
fn prompt_debug_is_redacted() {
    let prompt = AgentPrompt::from("private trading thesis".to_string());
    assert_eq!(format!("{prompt:?}"), "AgentPrompt(<redacted>)");
}

#[test]
fn chat_entry_debug_redacts_message_text() {
    let entry = AgentChatEntry::Message {
        role: AgentChatRole::User,
        text: "private portfolio question".to_string(),
        markdown: None,
        follow_ups: Vec::new(),
    };
    let debug = format!("{entry:?}");
    assert!(!debug.contains("private portfolio question"));
    assert!(debug.contains("<redacted>"));
}

#[test]
fn agent_uri_debug_is_redacted() {
    let uri = AgentUri::from("https://example.com/private?token=secret".to_string());
    let debug = format!("{uri:?}");

    assert_eq!(debug, "AgentUri(<redacted>)");
    assert!(!debug.contains("token=secret"));
}

#[test]
fn stored_session_debug_redacts_titles_drafts_and_messages() {
    let session = AgentStoredSession {
        id: 1,
        title: "private title".to_string(),
        created_at_ms: 1,
        updated_at_ms: 2,
        input: "private draft".to_string(),
        entries: vec![AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "private message".to_string(),
            markdown: None,
            follow_ups: Vec::new(),
        }],
        requested_model: Some("openrouter/auto".to_string()),
        runtime_model: Some("openrouter/auto".to_string()),
        context_tokens: Some(1_024),
        context_window: Some(2_000_000),
        total_tokens: None,
        total_cost_usd: None,
    };

    let debug = format!("{session:?}");

    for private in ["private title", "private draft", "private message"] {
        assert!(!debug.contains(private));
    }
    assert!(!debug.contains("openrouter/auto"));
    assert!(debug.contains("<redacted>"));
}

#[test]
fn persistence_result_debug_redacts_save_errors() {
    let result = AgentPersistenceResult::from(Err("private save detail".to_string()));
    let debug = format!("{result:?}");

    assert!(!debug.contains("private save detail"));
    assert_eq!(debug, "AgentPersistenceResult(Err(<redacted>))");
}

#[test]
fn legacy_persisted_sessions_default_context_metrics_to_unknown() {
    let session = serde_json::from_value::<PersistedAgentSession>(serde_json::json!({
        "id": 1,
        "title": "Legacy session",
        "created_at_ms": 1,
        "updated_at_ms": 2
    }))
    .expect("legacy Assistant session should deserialize");

    assert_eq!(session.requested_model, None);
    assert_eq!(session.runtime_model, None);
    assert_eq!(session.context_tokens, None);
    assert_eq!(session.context_window, None);
}
