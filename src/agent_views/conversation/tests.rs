use super::*;

#[test]
fn reasoning_duration_matches_thinking_disclosure_copy() {
    assert_eq!(reasoning_duration_label(0, false), "Thinking…");
    assert_eq!(reasoning_duration_label(63, true), "Thought for 1 second");
    assert_eq!(reasoning_duration_label(250, true), "Thought for 4 seconds");
}

#[test]
fn tool_trace_groups_calls_across_reasoning_within_one_turn() {
    let entries = vec![
        AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "Inspect my risk".to_string(),
            markdown: None,
            follow_ups: Vec::new(),
        },
        AgentChatEntry::Tool {
            call_id: "positions".to_string(),
            name: "kerosene_data".to_string(),
            detail: Some("Open positions".to_string()),
            finished: true,
            is_error: false,
            expanded: true,
        },
        AgentChatEntry::Reasoning {
            text: "Checking concentration".to_string(),
            elapsed_ticks: 1,
            finished: true,
            expanded: true,
        },
        AgentChatEntry::Tool {
            call_id: "risk".to_string(),
            name: "kerosene_risk".to_string(),
            detail: Some("5% adverse move".to_string()),
            finished: false,
            is_error: false,
            expanded: true,
        },
        AgentChatEntry::Message {
            role: AgentChatRole::Assistant,
            text: "Answer".to_string(),
            markdown: None,
            follow_ups: Vec::new(),
        },
        AgentChatEntry::Message {
            role: AgentChatRole::User,
            text: "Next question".to_string(),
            markdown: None,
            follow_ups: Vec::new(),
        },
    ];

    assert!(agent_tool_trace_starts_at(&entries, 1));
    assert!(!agent_tool_trace_starts_at(&entries, 3));
    let tools = agent_tool_trace_items(&entries, 1);
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name, "kerosene_data");
    assert_eq!(tools[1].name, "kerosene_risk");
    assert!(!tools[1].finished);
}

#[test]
fn tool_trace_uses_short_coding_style_action_labels() {
    assert_eq!(agent_tool_trace_action("kerosene_data"), "Read");
    assert_eq!(agent_tool_trace_action("kerosene_market_data"), "Fetch");
    assert_eq!(agent_tool_trace_action("kerosene_calculate"), "Calculate");
    assert_eq!(agent_tool_trace_action("kerosene_risk"), "Analyze");
    assert_eq!(agent_tool_trace_action("unknown_tool"), "Run");
}
