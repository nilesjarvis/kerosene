use super::*;
use crate::agent_state::{AgentPrompt, AgentStatus};

#[test]
fn streaming_deltas_share_one_assistant_entry() {
    let mut state = AgentState::default();
    state.append_assistant_delta("Hello");
    state.append_assistant_delta(" world");

    assert!(matches!(
        state.entries.as_slice(),
        [AgentChatEntry::Message {
            role: AgentChatRole::Assistant,
            text,
            ..
        }] if text == "Hello world"
    ));
}

#[test]
fn response_metadata_becomes_personalized_follow_ups_not_visible_answer_text() {
    let mut state = AgentState::default();
    state.append_assistant_delta(
        "BTC concentration is the main risk.\n\n<!-- KEROSENE_FOLLOW_UPS_V1\n[\"How would a 5% BTC drop affect my current margin buffer?\",\"Which BTC position contributes most to the concentration?\"]\nKEROSENE_FOLLOW_UPS_V1 -->",
    );

    assert!(state.finalize_assistant_response_metadata());

    let [
        AgentChatEntry::Message {
            text,
            markdown: Some(_),
            follow_ups,
            ..
        },
    ] = state.entries.as_slice()
    else {
        panic!("expected one finalized assistant response");
    };
    assert_eq!(text, "BTC concentration is the main risk.");
    assert_eq!(
        follow_ups,
        &[
            "How would a 5% BTC drop affect my current margin buffer?".to_string(),
            "Which BTC position contributes most to the concentration?".to_string(),
        ]
    );
    assert!(!text.contains("KEROSENE_FOLLOW_UPS"));
    assert_eq!(state.persisted_store().sessions[0].entries[0].text, *text);

    assert!(state.finish_assistant_presentation().is_some());
    assert!(matches!(
        state.entries.as_slice(),
        [AgentChatEntry::Message { follow_ups, .. }] if follow_ups.len() == 2
    ));
}

#[test]
fn malformed_or_absent_metadata_never_falls_back_to_generic_follow_ups() {
    let (plain, plain_follow_ups) = split_assistant_follow_ups("Visible answer");
    assert_eq!(plain, "Visible answer");
    assert!(plain_follow_ups.is_none());

    let (visible, malformed_follow_ups) =
        split_assistant_follow_ups("Visible answer\n<!-- KEROSENE_FOLLOW_UPS_V1\nnot-json");
    assert_eq!(visible, "Visible answer");
    assert!(malformed_follow_ups.is_some_and(|follow_ups| follow_ups.is_empty()));
}

#[test]
fn follow_up_section_boundaries_preserve_the_original_visible_prefix() {
    let valid = format!("{FOLLOW_UP_SECTION_START}[\" Next   question? \"]{FOLLOW_UP_SECTION_END}");
    let cases = [
        ("Plain answer  ".to_string(), None, None),
        (String::new(), None, None),
        (
            format!("Réponse λ\u{a0}\n{valid}\t"),
            Some("Réponse λ"),
            Some(vec!["Next question?"]),
        ),
        (valid.clone(), Some(""), Some(vec!["Next question?"])),
        (format!("Answer{valid}"), None, None),
        (format!("Answer\n{valid}\nVisible suffix"), None, None),
        (
            format!("Answer\n{FOLLOW_UP_SECTION_START}not-json"),
            Some("Answer"),
            Some(vec![]),
        ),
        (
            format!("Answer\n{FOLLOW_UP_SECTION_START}not-json{FOLLOW_UP_SECTION_END}"),
            Some("Answer"),
            Some(vec![]),
        ),
        (
            format!("Answer\n{FOLLOW_UP_SECTION_START}[42]{FOLLOW_UP_SECTION_END}"),
            Some("Answer"),
            Some(vec![]),
        ),
        (
            format!("Answer\n{valid}word{FOLLOW_UP_SECTION_START}"),
            None,
            None,
        ),
    ];

    for (response, expected_prefix, expected_follow_ups) in cases {
        let (visible, follow_ups) = split_assistant_follow_ups(&response);
        assert_eq!(visible, expected_prefix.unwrap_or(&response));
        assert_eq!(
            follow_ups,
            expected_follow_ups
                .map(|questions| questions.into_iter().map(str::to_string).collect())
        );
    }

    let first_section = format!("Answer\n{valid}");
    let response = format!("{first_section}\n{valid}");
    let (visible, follow_ups) = split_assistant_follow_ups(&response);
    assert_eq!(visible, first_section);
    assert_eq!(follow_ups, Some(vec!["Next question?".to_string()]));
}

#[test]
fn metadata_finalization_preserves_streamed_markdown_at_each_reveal_boundary() {
    for answer in ["", "## Résumé λ\n\n**BTC** → ETH"] {
        let response = format!(
            "{answer}\n\n{FOLLOW_UP_SECTION_START}[\"Next question?\"]{FOLLOW_UP_SECTION_END}"
        );
        let partial = answer.char_indices().nth(5).map_or(0, |(index, _)| index);
        for revealed in [0, partial, answer.len(), response.len()] {
            let mut state = AgentState::default();
            state.append_assistant_delta(&response[..revealed]);
            state.flush_assistant_stream();
            state.append_assistant_delta(&response[revealed..]);

            assert_eq!(
                state.finalize_assistant_response_metadata(),
                !answer.is_empty()
            );
            assert_eq!(state.stream.pending, answer[revealed.min(answer.len())..]);
            state.finish_assistant_presentation();

            let [
                AgentChatEntry::Message {
                    text,
                    markdown: Some(markdown),
                    follow_ups,
                    ..
                },
            ] = state.entries.as_slice()
            else {
                panic!("expected one finalized assistant response");
            };
            assert_eq!(text, answer);
            assert_eq!(follow_ups, &["Next question?".to_string()]);
            assert_eq!(
                format!("{:?}", markdown.items()),
                format!("{:?}", markdown::Content::parse(answer).items())
            );
            assert!(state.stream.pending.is_empty());
            assert_eq!(state.current_turn_has_text, !answer.is_empty());
        }
    }
}

#[test]
fn follow_up_metadata_is_normalized_deduplicated_and_bounded() {
    let long_question = format!("{}?", "x".repeat(MAX_FOLLOW_UP_CHARS + 40));
    let response = format!(
        "Answer\n\n{FOLLOW_UP_SECTION_START}\n{}\n{FOLLOW_UP_SECTION_END}",
        serde_json::json!([
            "  Compare BTC   with ETH?  ",
            "compare btc with eth?",
            long_question,
            "This third unique question must be dropped?",
        ])
    );

    let (visible, follow_ups) = split_assistant_follow_ups(&response);
    let follow_ups = follow_ups.expect("metadata marker should be recognized");

    assert_eq!(visible, "Answer");
    assert_eq!(follow_ups.len(), MAX_FOLLOW_UPS);
    assert_eq!(follow_ups[0], "Compare BTC with ETH?");
    assert_eq!(follow_ups[1].chars().count(), MAX_FOLLOW_UP_CHARS);
}

#[test]
fn streamed_reasoning_tracks_duration_and_stays_transient() {
    let mut state = AgentState {
        status: AgentStatus::Thinking,
        ..AgentState::default()
    };
    state.begin_reasoning();
    state.append_reasoning_delta("private portfolio reasoning");
    let _ = state.advance_assistant_stream();
    let _ = state.advance_assistant_stream();
    state.finish_reasoning();
    state.append_assistant_delta("Visible answer");

    let [reasoning, AgentChatEntry::Message { .. }] = state.entries.as_slice() else {
        panic!("expected a reasoning trace followed by the answer");
    };
    assert!(matches!(
        reasoning,
        AgentChatEntry::Reasoning {
            text,
            elapsed_ticks: 2,
            finished: true,
            expanded: true,
        } if text == "private portfolio reasoning"
    ));
    let debug = format!("{reasoning:?}");
    assert!(!debug.contains("private portfolio reasoning"));
    assert!(debug.contains("<redacted>"));

    let persisted = state.persisted_store();
    assert_eq!(persisted.sessions[0].entries.len(), 1);
    assert_eq!(persisted.sessions[0].entries[0].text, "Visible answer");
}

#[test]
fn reasoning_disclosure_can_be_collapsed() {
    let mut state = AgentState::default();
    state.begin_reasoning();
    state.append_reasoning_delta("Trace");
    state.toggle_reasoning(0);

    assert!(matches!(
        state.entries.as_slice(),
        [AgentChatEntry::Reasoning {
            expanded: false,
            ..
        }]
    ));
}

#[test]
fn streamed_assistant_markdown_builds_rich_blocks_incrementally() {
    let mut state = AgentState::default();
    state.append_assistant_delta("## Risk summary\n\n- **BTC** exposure\n\n");
    state.append_assistant_delta("```rust\nlet risk = 42;\n```\n");
    state.flush_assistant_stream();

    let [
        AgentChatEntry::Message {
            markdown: Some(markdown),
            ..
        },
    ] = state.entries.as_slice()
    else {
        panic!("expected one parsed assistant message");
    };

    assert!(matches!(
        markdown.items(),
        [
            markdown::Item::Heading(..),
            markdown::Item::List { .. },
            markdown::Item::CodeBlock { .. }
        ]
    ));
}

#[test]
fn stream_reveal_preserves_exact_markdown_and_utf8() {
    let mut state = AgentState::default();
    let response = "## Risk\n\nBTC → ETH  **spread**\n";
    state.append_assistant_delta(response);

    for _ in 0..64 {
        let _ = state.advance_assistant_stream();
        if state.stream.pending.is_empty() {
            break;
        }
    }

    let Some(AgentChatEntry::Message {
        text,
        markdown: Some(markdown),
        ..
    }) = state.entries.first()
    else {
        panic!("expected streamed Assistant message");
    };
    assert_eq!(text, response);
    assert!(!markdown.items().is_empty());
    assert!(state.stream.pending.is_empty());
}

#[test]
fn settled_partial_word_drains_and_completes() {
    let mut state = AgentState::default();
    state.append_assistant_delta("unfinished");

    let (changed, ready) = state.advance_assistant_stream();
    assert!(!changed);
    assert!(!ready);

    state.mark_assistant_transport_settled();
    let (changed, ready) = state.advance_assistant_stream();
    assert!(changed);
    assert!(ready);
    assert!(state.finish_assistant_presentation().is_some());
    assert!(state.assistant_entry_index.is_none());
}

#[test]
fn busy_assistant_keeps_the_activity_animation_ticking() {
    let mut state = AgentState {
        status: AgentStatus::Preparing,
        ..AgentState::default()
    };

    assert!(state.stream_needs_tick());
    let _ = state.advance_assistant_stream();
    assert_eq!(state.stream.activity_ticks, 1);

    state.begin_snapshot(AgentPrompt::from("next turn".to_string()));
    assert_eq!(state.stream.activity_ticks, 0);
}

#[test]
fn reveal_prefix_keeps_whitespace_and_waits_for_partial_words() {
    assert_eq!(reveal_prefix_len("hello world", 1, false), 6);
    assert_eq!(reveal_prefix_len("hello  \n\n", 1, false), 9);
    assert_eq!(reveal_prefix_len("partial", 1, false), 0);
    assert_eq!(reveal_prefix_len("partial", 1, true), 7);
    assert_eq!(reveal_prefix_len("éclair next", 1, false), 8);
}
