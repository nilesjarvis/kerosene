use super::*;
use crate::agent_runtime::AgentRuntimeEvent;
use crate::agent_state::AgentState;

#[test]
fn submit_requires_an_openrouter_key() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.openrouter_api_key.clear();
    terminal.agent.input = "Analyze my risk".to_string();

    let _ = terminal.update_agent(Message::AgentSubmit);

    assert_eq!(terminal.agent.status, AgentStatus::Error);
    assert!(terminal.agent.entries.is_empty());
}

#[test]
fn model_picker_reports_missing_key_without_starting_a_catalog_request() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.openrouter_api_key.clear();

    let _ = terminal.update_agent(Message::AgentToggleModelPicker);

    assert!(terminal.agent.model_picker_open);
    assert!(!terminal.agent.model_catalog_loading);
    assert!(
        terminal
            .agent
            .model_catalog_error
            .as_deref()
            .is_some_and(|error| error.contains("OpenRouter API key"))
    );
}

#[test]
fn model_catalog_results_are_scoped_to_the_openrouter_key_generation() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.openrouter_key_generation = 5;
    terminal.agent.model_catalog_loading = true;
    let model = crate::openrouter_api::OpenRouterModel {
        id: "vendor/tool-model".to_string(),
        name: "Vendor Tool Model".to_string(),
        context_length: Some(128_000),
        prompt_price_per_million_usd: Some(1.0),
        completion_price_per_million_usd: Some(2.0),
        reasoning_price_per_million_usd: None,
        request_price_usd: None,
        has_conditional_pricing: false,
        supports_image_input: true,
    };

    let _ = terminal.update_agent(Message::AgentModelCatalogLoaded(4, Ok(vec![model.clone()])));
    assert!(terminal.agent.model_catalog_loading);
    assert!(terminal.agent.model_catalog.is_empty());

    let _ = terminal.update_agent(Message::AgentModelCatalogLoaded(5, Ok(vec![model])));
    assert!(!terminal.agent.model_catalog_loading);
    assert_eq!(terminal.agent.model_catalog.len(), 1);
    assert_eq!(terminal.agent.model_catalog[0].id, "vendor/tool-model");
}

#[test]
fn detected_tool_capable_llama_cpp_can_be_selected_without_openrouter() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.openrouter_api_key.clear();
    terminal.agent.local_detection_generation = 3;
    terminal.agent.local_detection_loading = true;
    let server = crate::llama_cpp::LlamaCppServer {
        base_url: "http://127.0.0.1:35677/v1".to_string(),
        models: vec![crate::llama_cpp::LlamaCppModel {
            id: "local-model.gguf".to_string(),
            context_window: Some(30_720),
        }],
        supports_tools: true,
        supports_vision: true,
        supports_reasoning: true,
    };

    let _ = terminal.update_agent(Message::AgentLocalServerDetected(3, Ok(Some(server))));
    assert!(terminal.agent.model_picker_open);
    assert!(!terminal.agent.local_detection_loading);

    let _ = terminal.update_agent(Message::AgentProviderChanged(AssistantProvider::LlamaCpp));
    assert_eq!(terminal.assistant_provider, AssistantProvider::LlamaCpp);
    assert!(terminal.assistant_configured());
    assert_eq!(
        terminal.assistant_model_for_task().as_deref(),
        Some("local-model.gguf")
    );
    assert_eq!(
        terminal.assistant_model_supports_images("local-model.gguf"),
        Some(true)
    );
}

#[test]
fn stale_local_detection_result_does_not_replace_current_server() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.agent.local_detection_generation = 8;
    terminal.agent.local_detection_loading = true;

    let _ = terminal.update_agent(Message::AgentLocalServerDetected(7, Ok(None)));

    assert!(terminal.agent.local_detection_loading);
    assert!(terminal.agent.local_server.is_none());
}

#[test]
fn local_detection_only_invalidates_a_changed_active_local_runtime() {
    let (mut terminal, _) = TradingTerminal::boot();
    let original = crate::llama_cpp::LlamaCppServer {
        base_url: "http://127.0.0.1:35677/v1".to_string(),
        models: vec![crate::llama_cpp::LlamaCppModel {
            id: "original.gguf".to_string(),
            context_window: Some(32_768),
        }],
        supports_tools: true,
        supports_vision: false,
        supports_reasoning: true,
    };
    let mut replacement = original.clone();
    replacement.models[0].id = "replacement.gguf".to_string();
    let outcomes = [
        (Ok(Some(original.clone())), false),
        (Ok(Some(replacement)), true),
        (Ok(None), true),
        (Err("discovery unavailable".to_string()), true),
    ];

    for provider in [AssistantProvider::OpenRouter, AssistantProvider::LlamaCpp] {
        terminal.assistant_provider = provider;
        for connected in [false, true] {
            for current in [false, true] {
                for (result, changed) in &outcomes {
                    terminal.agent = AgentState {
                        runtime_generation: 40,
                        runtime_connected: connected,
                        status: AgentStatus::Thinking,
                        pending_prompt: Some("pending request".to_string().into()),
                        entries: vec![AgentChatEntry::Message {
                            role: AgentChatRole::User,
                            text: "pending request".to_string(),
                            markdown: None,
                            follow_ups: Vec::new(),
                        }],
                        workspace_actions_allowed: true,
                        local_detection_generation: 8,
                        local_detection_loading: true,
                        local_server: Some(original.clone()),
                        ..AgentState::default()
                    };

                    let _ = terminal.update_agent(Message::AgentLocalServerDetected(
                        if current { 8 } else { 7 },
                        result.clone(),
                    ));

                    let invalidated =
                        current && provider == AssistantProvider::LlamaCpp && connected && *changed;
                    assert_eq!(
                        terminal.agent.runtime_generation,
                        if invalidated { 41 } else { 40 }
                    );
                    assert_eq!(terminal.agent.runtime_connected, connected && !invalidated);
                    assert_eq!(terminal.agent.pending_prompt.is_some(), !invalidated);
                    assert_eq!(terminal.agent.workspace_actions_allowed, !invalidated);
                    assert_eq!(terminal.agent.needs_context_replay, invalidated);
                    assert_eq!(terminal.agent.local_detection_loading, !current);
                    assert_eq!(
                        terminal.agent.local_server,
                        if current {
                            result.clone().ok().flatten()
                        } else {
                            Some(original.clone())
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn settled_runtime_event_returns_ready_and_updates_usage() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.agent.runtime_generation = 4;
    terminal.agent.status = AgentStatus::Thinking;
    terminal.agent.append_assistant_delta("Visible answer");
    terminal.agent.flush_assistant_stream();

    let _ = terminal.update_agent(Message::AgentRuntimeEvent(AgentRuntimeEvent::Settled {
        generation: 4,
        total_tokens: Some(123),
        total_cost_usd: Some(0.0042),
        has_visible_text: Some(true),
    }));

    assert_eq!(terminal.agent.status, AgentStatus::Ready);
    assert_eq!(terminal.agent.total_tokens, Some(123));
    assert_eq!(terminal.agent.total_cost_usd, Some(0.0042));
}

#[test]
fn settled_response_waits_for_visual_queue_before_becoming_ready() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.agent.runtime_generation = 4;
    terminal.agent.status = AgentStatus::Thinking;
    terminal.agent.append_assistant_delta("Final answer");

    let _ = terminal.update_agent(Message::AgentRuntimeEvent(AgentRuntimeEvent::Settled {
        generation: 4,
        total_tokens: None,
        total_cost_usd: None,
        has_visible_text: Some(true),
    }));

    assert_eq!(terminal.agent.status, AgentStatus::Thinking);
    assert!(terminal.agent.stream_needs_tick());

    for _ in 0..8 {
        let _ = terminal.update_agent(Message::AgentStreamTick);
        if terminal.agent.status == AgentStatus::Ready {
            break;
        }
    }

    assert_eq!(terminal.agent.status, AgentStatus::Ready);
    assert_eq!(terminal.agent.stream.featured_entry_index, Some(0));
}

#[test]
fn runtime_reasoning_events_build_a_toggleable_trace() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.agent.runtime_generation = 4;
    terminal.agent.status = AgentStatus::Thinking;

    let _ = terminal.update_agent(Message::AgentRuntimeEvent(
        AgentRuntimeEvent::ReasoningStarted { generation: 4 },
    ));
    let _ = terminal.update_agent(Message::AgentRuntimeEvent(
        AgentRuntimeEvent::ReasoningDelta {
            generation: 4,
            delta: "Inspecting current evidence".to_string(),
        },
    ));
    let _ = terminal.update_agent(Message::AgentStreamTick);
    let _ = terminal.update_agent(Message::AgentRuntimeEvent(
        AgentRuntimeEvent::ReasoningFinished { generation: 4 },
    ));
    let _ = terminal.update_agent(Message::AgentToggleReasoning(0));

    assert!(matches!(
        terminal.agent.entries.as_slice(),
        [AgentChatEntry::Reasoning {
            text,
            elapsed_ticks: 1,
            finished: true,
            expanded: false,
        }] if text == "Inspecting current evidence"
    ));
}

#[test]
fn tool_trace_toggle_only_changes_the_requested_tool_group() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.agent.entries.push(AgentChatEntry::Tool {
        call_id: "call-1".to_string(),
        name: "kerosene_risk".to_string(),
        detail: None,
        finished: true,
        is_error: false,
        expanded: true,
    });

    let _ = terminal.update_agent(Message::AgentToggleToolTrace(1));
    assert!(matches!(
        &terminal.agent.entries[0],
        AgentChatEntry::Tool { expanded: true, .. }
    ));

    let _ = terminal.update_agent(Message::AgentToggleToolTrace(0));
    assert!(matches!(
        &terminal.agent.entries[0],
        AgentChatEntry::Tool {
            expanded: false,
            ..
        }
    ));
}

#[test]
fn new_chat_creates_a_saved_session_instead_of_erasing_the_previous_one() {
    let (mut terminal, _) = TradingTerminal::boot();
    let first_id = terminal.agent.active_session_id;
    terminal.agent.entries.push(AgentChatEntry::Message {
        role: AgentChatRole::User,
        text: "private first session".to_string(),
        markdown: None,
        follow_ups: Vec::new(),
    });

    let _ = terminal.update_agent(Message::AgentNewChat);

    assert_ne!(terminal.agent.active_session_id, first_id);
    assert!(terminal.agent.entries.is_empty());
    assert_eq!(terminal.agent.sessions.len(), 1);
    assert!(matches!(
        terminal.agent.sessions[0].entries.as_slice(),
        [AgentChatEntry::Message { text, .. }] if text == "private first session"
    ));
}

#[test]
fn assistant_sidebar_toggle_is_transient_and_reversible() {
    let (mut terminal, _) = TradingTerminal::boot();

    let _ = terminal.update_agent(Message::AgentToggleSidebar);
    assert!(terminal.agent.sidebar_collapsed);

    let _ = terminal.update_agent(Message::AgentToggleSidebar);
    assert!(!terminal.agent.sidebar_collapsed);
}

#[test]
fn closing_assistant_preserves_active_transcript() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.agent.window_id = Some(window::Id::unique());
    terminal.agent.entries.push(AgentChatEntry::Message {
        role: AgentChatRole::Assistant,
        text: "saved answer".to_string(),
        markdown: Some(Box::new(iced::widget::markdown::Content::parse(
            "saved answer",
        ))),
        follow_ups: Vec::new(),
    });

    terminal.close_agent_session();

    assert!(terminal.agent.window_id.is_none());
    assert_eq!(terminal.agent.entries.len(), 1);
    assert!(terminal.agent.needs_context_replay);
}

#[test]
fn closing_assistant_flushes_output_and_revokes_pending_runtime_work() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.config_clear_requested = true;
    terminal.agent = AgentState {
        window_id: Some(window::Id::unique()),
        runtime_generation: 41,
        snapshot_request_id: 17,
        runtime_connected: true,
        status: AgentStatus::Thinking,
        pending_prompt: Some("pending request".to_string().into()),
        workspace_actions_allowed: true,
        model_picker_open: true,
        model_search: "model query".to_string(),
        pnl_card_load_generation: 5,
        pnl_card_loading: true,
        pnl_card_drop_hovered: true,
        persistence_dirty: true,
        ..AgentState::default()
    };
    terminal.agent.entries.push(AgentChatEntry::Tool {
        call_id: "running-tool".to_string(),
        name: "kerosene_risk".to_string(),
        detail: None,
        finished: false,
        is_error: false,
        expanded: true,
    });
    terminal.agent.append_assistant_delta("Unfinished answer");
    assert!(terminal.agent.stream_needs_tick());

    terminal.close_agent_session();

    assert_eq!(terminal.agent.runtime_generation, 42);
    assert_eq!(terminal.agent.snapshot_request_id, 17);
    assert_eq!(terminal.agent.status, AgentStatus::Stopped);
    assert!(!terminal.agent.runtime_connected);
    assert!(!terminal.agent.workspace_actions_allowed);
    assert!(terminal.agent.pending_prompt.is_none());
    assert!(terminal.agent.needs_context_replay);
    assert!(terminal.agent.window_id.is_none());
    assert!(!terminal.agent.model_picker_open);
    assert!(terminal.agent.model_search.is_empty());
    assert_eq!(terminal.agent.pnl_card_load_generation, 6);
    assert!(!terminal.agent.pnl_card_loading);
    assert!(!terminal.agent.pnl_card_drop_hovered);
    assert!(!terminal.agent.persistence_dirty);
    assert!(!terminal.agent.stream_needs_tick());
    assert!(matches!(
        terminal.agent.entries.as_slice(),
        [
            AgentChatEntry::Tool { finished: true, is_error: true, .. },
            AgentChatEntry::Message { role: AgentChatRole::Assistant, text, markdown: Some(_), .. },
        ] if text == "Unfinished answer"
    ));
}

#[test]
fn config_clear_reset_discards_sessions_and_preserves_the_save_barrier() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.agent.window_id = Some(window::Id::unique());
    terminal.agent.runtime_generation = 7;
    terminal.agent.snapshot_request_id = 11;
    terminal.agent.persistence_generation = 13;
    terminal.agent.persistence_in_flight = true;
    terminal.agent.persistence_dirty = true;
    terminal.agent.entries.push(AgentChatEntry::Message {
        role: AgentChatRole::User,
        text: "private prompt".to_string(),
        markdown: None,
        follow_ups: Vec::new(),
    });
    assert!(terminal.agent.create_session(100));
    terminal.agent.input = "private draft".to_string();
    let previous_runtime_generation = terminal.agent.runtime_generation;

    let _ = terminal.prepare_agent_for_config_clear();

    assert!(terminal.agent.window_id.is_none());
    assert!(terminal.agent.sessions.is_empty());
    assert!(terminal.agent.entries.is_empty());
    assert!(terminal.agent.input.is_empty());
    assert_eq!(
        terminal.agent.runtime_generation,
        previous_runtime_generation.wrapping_add(1)
    );
    assert_eq!(terminal.agent.snapshot_request_id, 12);
    assert_eq!(terminal.agent.persistence_generation, 13);
    assert!(terminal.agent.persistence_in_flight);
    assert!(!terminal.agent.persistence_dirty);
}

#[test]
fn config_clear_waits_for_assistant_save_without_queuing_another_save() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.config_clear_requested = true;
    terminal.agent.persistence_generation = 5;
    terminal.agent.persistence_in_flight = true;
    terminal.agent.persistence_dirty = true;

    let _ = terminal.prepare_agent_for_config_clear();
    let _ = terminal.handle_agent_sessions_saved(5, Ok(()));

    assert!(terminal.config_clear_requested);
    assert!(!terminal.agent.persistence_in_flight);
    assert!(!terminal.agent.persistence_dirty);
}

#[test]
fn runtime_context_updates_are_scoped_to_the_active_generation() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.agent.runtime_generation = 4;
    terminal
        .agent
        .prepare_context_for_model("anthropic/claude-sonnet-4.5");

    let _ = terminal.update_agent(Message::AgentRuntimeEvent(
        AgentRuntimeEvent::ModelContext {
            generation: 4,
            model: Some("anthropic/claude-sonnet-4.5".to_string()),
            context_window: Some(1_000_000),
        },
    ));
    let _ = terminal.update_agent(Message::AgentRuntimeEvent(
        AgentRuntimeEvent::ContextUsage {
            generation: 4,
            context_tokens: Some(25_000),
            context_window: Some(1_000_000),
        },
    ));

    assert_eq!(terminal.agent.context_tokens, Some(25_000));
    assert_eq!(terminal.agent.context_window, Some(1_000_000));

    let _ = terminal.update_agent(Message::AgentRuntimeEvent(
        AgentRuntimeEvent::ContextUsage {
            generation: 3,
            context_tokens: Some(999_999),
            context_window: Some(1_000_000),
        },
    ));
    assert_eq!(terminal.agent.context_tokens, Some(25_000));

    let _ = terminal.update_agent(Message::AgentRuntimeEvent(
        AgentRuntimeEvent::ContextUsage {
            generation: 4,
            context_tokens: None,
            context_window: Some(1_000_000),
        },
    ));
    assert_eq!(terminal.agent.context_tokens, None);
}
