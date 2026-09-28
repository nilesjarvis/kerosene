use super::*;

#[test]
fn sessions_can_be_created_and_switched_without_losing_transcripts() {
    let mut state = AgentState::default();
    let first_id = state.active_session_id;
    state.note_user_prompt("Review my BTC risk", 10);
    state.entries.push(AgentChatEntry::Message {
        role: AgentChatRole::User,
        text: "Review my BTC risk".to_string(),
        markdown: None,
        follow_ups: Vec::new(),
    });

    assert!(state.create_session(20));
    let second_id = state.active_session_id;
    assert_ne!(first_id, second_id);
    assert!(state.entries.is_empty());
    state.note_user_prompt("Show my best trades", 30);
    state.entries.push(AgentChatEntry::Message {
        role: AgentChatRole::User,
        text: "Show my best trades".to_string(),
        markdown: None,
        follow_ups: Vec::new(),
    });

    assert!(state.switch_session(first_id));
    assert_eq!(state.active_session_id, first_id);
    assert_eq!(state.active_session_title, "Review my BTC risk");
    assert!(matches!(
        state.entries.as_slice(),
        [AgentChatEntry::Message { text, .. }] if text == "Review my BTC risk"
    ));
    assert!(state.needs_context_replay);
}

#[test]
fn persisted_sessions_restore_markdown_and_active_selection() {
    let mut state = AgentState::default();
    state.note_user_prompt("Saved session", 10);
    state.entries.push(AgentChatEntry::Message {
        role: AgentChatRole::User,
        text: "private question".to_string(),
        markdown: None,
        follow_ups: Vec::new(),
    });
    state.entries.push(AgentChatEntry::Message {
        role: AgentChatRole::Assistant,
        text: "## Saved answer".to_string(),
        markdown: Some(Box::new(markdown::Content::parse("## Saved answer"))),
        follow_ups: Vec::new(),
    });
    state.prepare_context_for_model("openrouter/auto");
    state.update_runtime_model_context(Some("openrouter/auto".to_string()), Some(2_000_000));
    state.replace_context_usage(Some(12_000), Some(2_000_000));
    let active_id = state.active_session_id;

    let restored = AgentState::from_persisted_store(state.persisted_store());

    assert_eq!(restored.active_session_id, active_id);
    assert_eq!(restored.entries.len(), 2);
    assert!(matches!(
        &restored.entries[1],
        AgentChatEntry::Message {
            role: AgentChatRole::Assistant,
            markdown: Some(_),
            ..
        }
    ));
    assert!(restored.needs_context_replay);
    assert_eq!(
        restored.context_metrics_for_model("openrouter/auto"),
        (Some("openrouter/auto"), Some(12_000), Some(2_000_000))
    );
}

#[test]
fn restored_session_runtime_prompt_replays_bounded_history() {
    let mut state = AgentState::default();
    state.entries.push(AgentChatEntry::Message {
        role: AgentChatRole::User,
        text: "Earlier private question".to_string(),
        markdown: None,
        follow_ups: Vec::new(),
    });
    state.entries.push(AgentChatEntry::Message {
        role: AgentChatRole::Assistant,
        text: "Earlier private answer".to_string(),
        markdown: Some(Box::new(markdown::Content::parse("Earlier private answer"))),
        follow_ups: Vec::new(),
    });
    state.needs_context_replay = true;

    let prompt = state.runtime_prompt("Follow up now");

    assert!(prompt.as_str().contains("Earlier private question"));
    assert!(prompt.as_str().contains("Earlier private answer"));
    assert!(prompt.as_str().contains("Follow up now"));
    assert_eq!(format!("{prompt:?}"), "AgentPrompt(<redacted>)");
}

#[test]
fn generated_session_titles_respect_the_character_limit() {
    let title = session_title(&"a".repeat(MAX_SESSION_TITLE_CHARS + 10));

    assert_eq!(title.chars().count(), MAX_SESSION_TITLE_CHARS);
    assert!(title.ends_with('…'));
}

#[test]
fn context_metrics_follow_each_session_and_reset_for_a_new_model() {
    let mut state = AgentState::default();
    let first_id = state.active_session_id;
    state.prepare_context_for_model("openrouter/auto");
    state.update_runtime_model_context(Some("openrouter/auto".to_string()), Some(2_000_000));
    state.replace_context_usage(Some(12_000), Some(2_000_000));

    assert!(state.create_session(20));
    state.prepare_context_for_model("anthropic/claude-sonnet-4.5");
    state.update_runtime_model_context(
        Some("anthropic/claude-sonnet-4.5".to_string()),
        Some(1_000_000),
    );
    state.replace_context_usage(Some(4_000), Some(1_000_000));

    assert!(state.switch_session(first_id));
    assert_eq!(
        state.context_metrics_for_model("openrouter/auto"),
        (Some("openrouter/auto"), Some(12_000), Some(2_000_000))
    );

    state.prepare_context_for_model("google/gemini-2.5-pro");
    assert_eq!(
        state.context_metrics_for_model("google/gemini-2.5-pro"),
        (None, None, None)
    );
}

#[test]
fn session_entry_limits_preserve_save_and_restore_filter_order() {
    let mut state = AgentState::default();
    for index in 0..MAX_PERSISTED_ENTRIES_PER_SESSION + 2 {
        state.entries.extend([
            AgentChatEntry::Message {
                role: AgentChatRole::User,
                text: format!("message {index}"),
                markdown: None,
                follow_ups: Vec::new(),
            },
            AgentChatEntry::Message {
                role: AgentChatRole::Assistant,
                text: String::new(),
                markdown: None,
                follow_ups: Vec::new(),
            },
            AgentChatEntry::Reasoning {
                text: "transient reasoning".into(),
                elapsed_ticks: 1,
                finished: true,
                expanded: false,
            },
            AgentChatEntry::Tool {
                call_id: "call".into(),
                name: "kerosene_data".into(),
                detail: Some("transient detail".into()),
                finished: true,
                is_error: false,
                expanded: false,
            },
        ]);
    }
    let mut store = state.persisted_store();
    let saved = &store.sessions[0].entries;
    assert_eq!(saved.len(), MAX_PERSISTED_ENTRIES_PER_SESSION);
    for (index, entry) in saved.iter().enumerate() {
        assert!(matches!(entry.role, PersistedAgentRole::User));
        assert_eq!(entry.text, format!("message {}", index + 2));
    }

    // Restoration applies the entry cap before dropping empty messages.
    store.sessions[0].entries = (0..MAX_PERSISTED_ENTRIES_PER_SESSION + 2)
        .map(|index| PersistedAgentEntry {
            role: PersistedAgentRole::Assistant,
            text: if index == 2 || index == MAX_PERSISTED_ENTRIES_PER_SESSION + 1 {
                String::new()
            } else {
                format!("answer {index}")
            },
        })
        .collect();
    let restored = AgentState::from_persisted_store(store);
    assert_eq!(
        restored.entries.len(),
        MAX_PERSISTED_ENTRIES_PER_SESSION - 2
    );
    for (index, entry) in restored.entries.iter().enumerate() {
        assert!(matches!(entry, AgentChatEntry::Message {
            role: AgentChatRole::Assistant,
            text,
            markdown: Some(_),
            follow_ups,
        } if text == &format!("answer {}", index + 3) && follow_ups.is_empty()));
    }
    assert!(restored.needs_context_replay);
    assert_eq!(
        restored.stream.featured_entry_index,
        Some(restored.entries.len() - 1)
    );
}

#[test]
fn replay_suffix_preserves_unicode_and_latest_message_order() {
    for (limit, expected) in [
        (0, ""),
        (1, "\n"),
        (2, "x\n"),
        (3, "🦀x\n"),
        (4, "中🦀x\n"),
        (5, "é中🦀x\n"),
        (6, "é中🦀x\n"),
    ] {
        assert_eq!(trailing_text("é中🦀x\n", limit), expected);
        assert_eq!(trailing_text("", limit), "");
    }
    let entries = [
        AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "discarded older message".into(),
            markdown: None,
            follow_ups: Vec::new(),
        },
        AgentChatEntry::Message {
            role: AgentChatRole::Assistant,
            text: format!(
                "discarded prefix{}",
                "é中🦀".repeat(MAX_REPLAY_CONTEXT_CHARS)
            ),
            markdown: None,
            follow_ups: Vec::new(),
        },
        AgentChatEntry::Reasoning {
            text: "transient reasoning".into(),
            elapsed_ticks: 0,
            finished: true,
            expanded: false,
        },
        AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "latest".into(),
            markdown: None,
            follow_ups: Vec::new(),
        },
    ];
    let transcript = replay_transcript(&entries);
    // Existing accounting uses 16 characters for a user wrapper and 26 for
    // an assistant wrapper, then joins retained messages with two newlines.
    let suffix = "é中🦀".repeat((MAX_REPLAY_CONTEXT_CHARS - 6 - 16 - 26) / 3);
    assert_eq!(
        transcript,
        format!("<assistant>\n{suffix}\n</assistant>\n\n<user>\nlatest\n</user>")
    );
}
