use super::*;
use crate::agent_state::AgentChatEntry;
use crate::annotations::{Annotation, AnnotationKind, AnnotationStyle, LineStyle};
use crate::chart_state::ChartInstance;
use crate::timeframe::Timeframe;

fn terminal_with_running_tool() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal
        .charts
        .insert(7, ChartInstance::new(7, "BTC".to_string(), Timeframe::H1));
    terminal.agent.entries.push(AgentChatEntry::Tool {
        call_id: "call-1".to_string(),
        name: "kerosene_set_chart_indicators".to_string(),
        detail: None,
        finished: false,
        is_error: false,
        expanded: true,
    });
    terminal.agent.workspace_actions_allowed = true;
    terminal
}

fn terminal_with_running_drawing_tool() -> TradingTerminal {
    let mut terminal = terminal_with_running_tool();
    let Some(AgentChatEntry::Tool { name, .. }) = terminal.agent.entries.last_mut() else {
        panic!("running tool entry");
    };
    *name = "kerosene_manage_chart_drawings".to_string();
    terminal
}

fn action_payload(enabled: bool) -> String {
    serde_json::json!({
        "version": HOST_ACTION_VERSION,
        "tool_call_id": "call-1",
        "action": {
            "type": "set_chart_indicators",
            "chart_ids": [7],
            "changes": [{ "indicator_id": "tf_ema_50", "enabled": enabled }],
        }
    })
    .to_string()
}

fn drawing_payload(operations: serde_json::Value) -> String {
    serde_json::json!({
        "version": HOST_ACTION_VERSION,
        "tool_call_id": "call-1",
        "action": {
            "type": "manage_chart_drawings",
            "operations": operations,
        }
    })
    .to_string()
}

fn assert_annotations_mirrored(chart: &ChartInstance) {
    assert_eq!(chart.chart.annotations.len(), chart.annotations.len());
    for (canvas, persisted) in chart.chart.annotations.iter().zip(&chart.annotations) {
        assert_eq!(canvas.id, persisted.id);
        assert_eq!(canvas.kind, persisted.kind);
        assert_eq!(canvas.style, persisted.style);
    }
}

#[test]
fn assistant_indicator_action_is_idempotent() {
    let mut terminal = terminal_with_running_tool();
    let (first, _task) = terminal.handle_agent_host_action(&action_payload(true));
    let (second, _task) = terminal.handle_agent_host_action(&action_payload(true));
    let first: serde_json::Value = serde_json::from_str(&first).expect("first result");
    let second: serde_json::Value = serde_json::from_str(&second).expect("second result");

    assert_eq!(first["success"], true);
    assert_eq!(first["charts"][0]["changes"][0]["outcome"], "changed");
    assert_eq!(second["success"], true);
    assert_eq!(second["charts"][0]["changes"][0]["outcome"], "already_set");
    assert!(terminal.charts[&7].macro_indicators.tf_ema_50);
}

#[test]
fn quick_trade_is_not_available_to_the_assistant() {
    let mut terminal = terminal_with_running_tool();
    let payload = serde_json::json!({
        "version": HOST_ACTION_VERSION,
        "tool_call_id": "call-1",
        "action": {
            "type": "set_chart_indicators",
            "chart_ids": [7],
            "changes": [{ "indicator_id": "quick_trade", "enabled": true }],
        }
    })
    .to_string();

    let (result, _task) = terminal.handle_agent_host_action(&payload);
    let result: serde_json::Value = serde_json::from_str(&result).expect("result");
    assert_eq!(result["success"], false);
    assert_eq!(result["error"]["code"], "unsupported_indicator");
    assert!(!terminal.charts[&7].macro_indicators.show_quick_trade);
}

#[test]
fn a_failed_dependency_preflight_does_not_apply_part_of_the_batch() {
    let mut terminal = terminal_with_running_tool();
    terminal.hydromancer_api_key = String::new().into();
    let payload = serde_json::json!({
        "version": HOST_ACTION_VERSION,
        "tool_call_id": "call-1",
        "action": {
            "type": "set_chart_indicators",
            "chart_ids": [7],
            "changes": [
                { "indicator_id": "tf_ema_50", "enabled": true },
                { "indicator_id": "funding_rate", "enabled": true },
            ],
        }
    })
    .to_string();

    let (result, _task) = terminal.handle_agent_host_action(&payload);
    let result: serde_json::Value = serde_json::from_str(&result).expect("result");

    assert_eq!(result["success"], false);
    assert_eq!(result["error"]["code"], "dependency_missing");
    assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
    assert!(!terminal.charts[&7].macro_indicators.show_funding_rate);
}

#[test]
fn stale_tool_call_cannot_mutate_a_chart() {
    let mut terminal = terminal_with_running_tool();
    let payload = action_payload(true).replace("call-1", "stale-call");
    let (result, _task) = terminal.handle_agent_host_action(&payload);
    let result: serde_json::Value = serde_json::from_str(&result).expect("result");

    assert_eq!(result["success"], false);
    assert_eq!(result["action"], "set_chart_indicators");
    assert_eq!(result["error"]["code"], "inactive_tool_call");
    assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
}

#[test]
fn aborted_turn_cannot_mutate_a_chart() {
    let mut terminal = terminal_with_running_tool();
    let _task = terminal.update_agent(Message::AgentAbort);
    let (result, _task) = terminal.handle_agent_host_action(&action_payload(true));
    let result: serde_json::Value = serde_json::from_str(&result).expect("result");

    assert_eq!(result["success"], false);
    assert_eq!(result["error"]["code"], "inactive_tool_call");
    assert!(
        !terminal
            .agent
            .has_running_tool_call("call-1", "kerosene_set_chart_indicators")
    );
    assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
}

#[test]
fn unknown_action_fields_are_rejected() {
    let mut terminal = terminal_with_running_tool();
    let payload =
        action_payload(true).replace("\"changes\":[", "\"unexpected\":true,\"changes\":[");
    let (result, _task) = terminal.handle_agent_host_action(&payload);
    let result: serde_json::Value = serde_json::from_str(&result).expect("result");

    assert_eq!(result["success"], false);
    assert_eq!(result["error"]["code"], "invalid_request");
    assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
}

#[test]
fn admission_failures_preserve_the_indicator_error_envelope() {
    let mut terminal = terminal_with_running_tool();
    for (payload, code, message) in [
        (
            " ".repeat(MAX_HOST_ACTION_BYTES + 1),
            "request_too_large",
            "The workspace action request exceeded Kerosene's size limit",
        ),
        (
            " ".repeat(MAX_HOST_ACTION_BYTES),
            "invalid_request",
            "The workspace action request did not match the supported contract",
        ),
        (
            "{\"unexpected\":true}".to_string(),
            "invalid_request",
            "The workspace action request did not match the supported contract",
        ),
    ] {
        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");
        assert_eq!(
            result,
            serde_json::json!({
                "success": false, "action": "set_chart_indicators", "charts": [],
                "persistence_scheduled": false, "warnings": [],
                "error": { "code": code, "message": message },
            })
        );
        assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
        assert!(terminal.charts[&7].annotations.is_empty());
    }
}

#[test]
fn indicator_count_limits_precede_duplicate_validation() {
    for chart_count in [0, 1, MAX_TARGET_CHARTS, MAX_TARGET_CHARTS + 1] {
        for change_count in [0, 1, MAX_INDICATOR_CHANGES, MAX_INDICATOR_CHANGES + 1] {
            let mut terminal = terminal_with_running_tool();
            let payload = serde_json::json!({
                "version": HOST_ACTION_VERSION,
                "tool_call_id": "call-1",
                "action": {
                    "type": "set_chart_indicators",
                    "chart_ids": vec![7; chart_count],
                    "changes": vec![serde_json::json!({ "indicator_id": "tf_ema_50", "enabled": true }); change_count],
                },
            }).to_string();
            let expected_error = match (chart_count, change_count) {
                (0, _) | (33, _) => Some("invalid_chart_count"),
                (_, 0) | (_, 33) => Some("invalid_change_count"),
                (32, _) => Some("duplicate_chart"),
                (_, 32) => Some("duplicate_indicator"),
                (1, 1) => None,
                _ => panic!("unexpected boundary fixture"),
            };

            let (result, _task) = terminal.handle_agent_host_action(&payload);
            let result: serde_json::Value = serde_json::from_str(&result).expect("result");
            assert_eq!(result["error"]["code"].as_str(), expected_error);
            assert_eq!(result["success"], expected_error.is_none());
            assert_eq!(
                terminal.charts[&7].macro_indicators.tf_ema_50,
                expected_error.is_none()
            );
        }
    }
}

#[test]
fn action_persistence_preserves_pause_flags_and_idempotent_retries() {
    for drawings in [false, true] {
        for pause_flags in 0..8 {
            for pending_save in [false, true] {
                let mut terminal = if drawings {
                    terminal_with_running_drawing_tool()
                } else {
                    terminal_with_running_tool()
                };
                terminal.secret_migration_save_blocked = pause_flags & 1 != 0;
                terminal.config_clear_requested = pause_flags & 2 != 0;
                terminal.config_cleared_this_session = pause_flags & 4 != 0;
                terminal.config_save_due_at = pending_save.then(std::time::Instant::now);
                terminal.secret_store_status = None;
                let payload = if drawings {
                    drawing_payload(serde_json::json!([{
                        "operation": "add", "chart_id": 7,
                        "drawing": { "type": "horizontal_level", "price": 60_000.0 },
                    }]))
                } else {
                    action_payload(true)
                };

                let (first, _task) = terminal.handle_agent_host_action(&payload);
                let first: serde_json::Value = serde_json::from_str(&first).expect("first result");
                assert_eq!(first["success"], true);
                assert_eq!(first["persistence_scheduled"], pause_flags == 0);
                assert_eq!(
                    terminal.config_save_due_at.is_some(),
                    pending_save || pause_flags == 0
                );
                assert_eq!(terminal.secret_store_status.is_some(), pause_flags == 1);
                let expected_warnings = if pause_flags == 0 {
                    Vec::new()
                } else {
                    vec![format!(
                        "{} changes are active for this session, but configuration persistence is paused",
                        if drawings { "Drawing" } else { "Indicator" }
                    )]
                };
                assert_eq!(first["warnings"], serde_json::json!(expected_warnings));
                if drawings {
                    assert_eq!(terminal.charts[&7].annotations.len(), 1);
                } else {
                    assert!(terminal.charts[&7].macro_indicators.tf_ema_50);
                }
                let deadline = terminal.config_save_due_at;

                let (retry, _task) = terminal.handle_agent_host_action(&payload);
                let retry: serde_json::Value = serde_json::from_str(&retry).expect("retry result");
                assert_eq!(retry["success"], true);
                assert_eq!(retry["persistence_scheduled"], false);
                assert_eq!(retry["warnings"], serde_json::json!([]));
                assert_eq!(terminal.config_save_due_at, deadline);
            }
        }
    }
}

#[test]
fn drawing_candidates_preserve_repeated_chart_order_and_atomic_failures() {
    for failure in [None, Some("chart_not_found"), Some("drawing_not_found")] {
        let mut terminal = terminal_with_running_drawing_tool();
        terminal
            .charts
            .insert(9, ChartInstance::new(9, "ETH".to_string(), Timeframe::H1));
        terminal.config_save_due_at = None;
        let mut operations = vec![
            serde_json::json!({ "operation": "add", "chart_id": 7, "drawing": { "type": "horizontal_level", "price": 60_000.0 } }),
            serde_json::json!({ "operation": "add", "chart_id": 9, "drawing": { "type": "horizontal_level", "price": 3_000.0 } }),
            serde_json::json!({ "operation": "remove", "chart_id": 7, "drawing_id": 0 }),
            serde_json::json!({ "operation": "add", "chart_id": 7, "drawing": { "type": "horizontal_level", "price": 61_000.0 } }),
        ];
        if let Some(failure) = failure {
            operations.push(serde_json::json!({
                "operation": "remove", "chart_id": if failure == "chart_not_found" { 99 } else { 9 }, "drawing_id": 42,
            }));
        }
        let (result, _task) =
            terminal.handle_agent_host_action(&drawing_payload(serde_json::json!(operations)));
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");
        assert_eq!(result["error"]["code"].as_str(), failure);
        for chart_id in [7, 9] {
            let chart = &terminal.charts[&chart_id];
            assert_annotations_mirrored(chart);
            if failure.is_some() {
                assert!(chart.annotations.is_empty());
                assert_eq!(chart.next_annotation_id, 0);
                assert_eq!(terminal.config_save_due_at, None);
            } else {
                let expected_id = u64::from(chart_id == 7);
                assert_eq!(chart.annotations.len(), 1);
                assert_eq!(chart.annotations[0].id, expected_id);
                assert_eq!(chart.next_annotation_id, expected_id + 1);
            }
        }
        if failure.is_none() {
            let rows = result["operations"].as_array().expect("operation results");
            for (index, (chart_id, drawing_id, outcome)) in [
                (7, 0, "created"),
                (9, 0, "created"),
                (7, 0, "removed"),
                (7, 1, "created"),
            ]
            .into_iter()
            .enumerate()
            {
                assert_eq!(rows[index]["operation_index"], index);
                assert_eq!(rows[index]["chart_id"], chart_id);
                assert_eq!(rows[index]["drawing_id"], drawing_id);
                assert_eq!(rows[index]["outcome"], outcome);
            }
        }
    }
}

#[test]
fn assistant_can_create_every_supported_drawing_type() {
    let mut terminal = terminal_with_running_drawing_tool();
    let payload = drawing_payload(serde_json::json!([
        { "operation": "add", "chart_id": 7, "drawing": { "type": "horizontal_level", "price": 60_000.0 } },
        { "operation": "add", "chart_id": 7, "drawing": { "type": "vertical_line", "time_ms": 1_700_000_000_000_u64 } },
        { "operation": "add", "chart_id": 7, "drawing": { "type": "trend_line", "start": { "time_ms": 1_700_000_000_000_u64, "price": 59_000.0 }, "end": { "time_ms": 1_700_003_600_000_u64, "price": 61_000.0 } } },
        { "operation": "add", "chart_id": 7, "drawing": { "type": "ray", "start": { "time_ms": 1_700_000_000_000_u64, "price": 58_000.0 }, "end": { "time_ms": 1_700_003_600_000_u64, "price": 60_000.0 } } },
        { "operation": "add", "chart_id": 7, "drawing": { "type": "extended_line", "start": { "time_ms": 1_700_000_000_000_u64, "price": 57_000.0 }, "end": { "time_ms": 1_700_003_600_000_u64, "price": 59_000.0 } } },
        { "operation": "add", "chart_id": 7, "drawing": { "type": "rectangle", "a": { "time_ms": 1_700_000_000_000_u64, "price": 55_000.0 }, "b": { "time_ms": 1_700_003_600_000_u64, "price": 56_000.0 }, "style": { "color": "purple", "width": 2.5, "line_style": "dashed", "label": "Demand zone" } } },
        { "operation": "add", "chart_id": 7, "drawing": { "type": "measure", "start": { "time_ms": 1_700_000_000_000_u64, "price": 60_000.0 }, "end": { "time_ms": 1_700_003_600_000_u64, "price": 62_000.0 } } },
        { "operation": "add", "chart_id": 7, "drawing": { "type": "fib_retracement", "a": { "time_ms": 1_700_000_000_000_u64, "price": 50_000.0 }, "b": { "time_ms": 1_700_003_600_000_u64, "price": 65_000.0 } } },
        { "operation": "add", "chart_id": 7, "drawing": { "type": "fib_extension", "a": { "time_ms": 1_700_000_000_000_u64, "price": 50_000.0 }, "b": { "time_ms": 1_700_003_600_000_u64, "price": 65_000.0 }, "c": { "time_ms": 1_700_007_200_000_u64, "price": 60_000.0 } } }
    ]));

    let (result, _task) = terminal.handle_agent_host_action(&payload);
    let result: serde_json::Value = serde_json::from_str(&result).expect("result");
    let chart = &terminal.charts[&7];

    assert_eq!(result["success"], true);
    assert_eq!(result["operations"].as_array().map(Vec::len), Some(9));
    for (index, expected_type) in [
        "horizontal_level",
        "vertical_line",
        "trend_line",
        "ray",
        "extended_line",
        "rectangle",
        "measure",
        "fib_retracement",
        "fib_extension",
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(result["operations"][index]["drawing_type"], expected_type);
    }
    assert!(
        result["operations"]
            .as_array()
            .is_some_and(|rows| rows.iter().all(|row| row["outcome"] == "created"))
    );
    assert_eq!(chart.annotations.len(), 9);
    assert_annotations_mirrored(chart);
    assert_eq!(chart.next_annotation_id, 9);
    let rectangle = chart
        .annotations
        .iter()
        .find(|annotation| matches!(annotation.kind, AnnotationKind::Rectangle { .. }))
        .expect("rectangle");
    assert_eq!(rectangle.style.label.as_deref(), Some("Demand zone"));
    assert_eq!(rectangle.style.width, 2.5);
    assert_eq!(rectangle.style.line_style, LineStyle::Dashed);
    assert!(!rectangle.style.locked);
    assert!(rectangle.style.visible);
}

#[test]
fn assistant_drawing_add_is_idempotent() {
    let mut terminal = terminal_with_running_drawing_tool();
    let payload = drawing_payload(serde_json::json!([{
        "operation": "add",
        "chart_id": 7,
        "drawing": { "type": "horizontal_level", "price": 60_000.0 }
    }]));

    let (first, _task) = terminal.handle_agent_host_action(&payload);
    let (second, _task) = terminal.handle_agent_host_action(&payload);
    let first: serde_json::Value = serde_json::from_str(&first).expect("first result");
    let second: serde_json::Value = serde_json::from_str(&second).expect("second result");

    assert_eq!(first["operations"][0]["outcome"], "created");
    assert_eq!(second["operations"][0]["outcome"], "already_present");
    assert_eq!(first["operations"][0]["drawing_id"], 0);
    assert_eq!(second["operations"][0]["drawing_id"], 0);
    assert_eq!(terminal.charts[&7].annotations.len(), 1);
    assert_eq!(terminal.charts[&7].next_annotation_id, 1);
}

#[test]
fn failed_drawing_batch_is_atomic() {
    let mut terminal = terminal_with_running_drawing_tool();
    let locked = Annotation {
        id: 4,
        kind: AnnotationKind::HorizontalLevel { price: 55_000.0 },
        style: AnnotationStyle {
            locked: true,
            ..AnnotationStyle::default()
        },
    };
    {
        let chart = terminal.charts.get_mut(&7).expect("chart");
        chart.annotations.push(locked.clone());
        chart.chart.annotations = chart.annotations.clone();
        chart.next_annotation_id = 5;
    }
    let payload = drawing_payload(serde_json::json!([
        { "operation": "add", "chart_id": 7, "drawing": { "type": "horizontal_level", "price": 60_000.0 } },
        { "operation": "remove", "chart_id": 7, "drawing_id": 4 }
    ]));

    let (result, _task) = terminal.handle_agent_host_action(&payload);
    let result: serde_json::Value = serde_json::from_str(&result).expect("result");
    let chart = &terminal.charts[&7];

    assert_eq!(result["success"], false);
    assert_eq!(result["error"]["code"], "drawing_locked");
    assert_eq!(chart.annotations.len(), 1);
    assert_eq!(chart.annotations[0].id, locked.id);
    assert_eq!(chart.annotations[0].kind, locked.kind);
    assert_eq!(chart.annotations[0].style, locked.style);
    assert_annotations_mirrored(chart);
    assert_eq!(chart.next_annotation_id, 5);
}

#[test]
fn assistant_remove_clears_selection_and_canvas_copy() {
    let mut terminal = terminal_with_running_drawing_tool();
    let annotation = Annotation {
        id: 3,
        kind: AnnotationKind::VerticalLine {
            time: 1_700_000_000_000,
        },
        style: AnnotationStyle::default(),
    };
    {
        let chart = terminal.charts.get_mut(&7).expect("chart");
        chart.annotations.push(annotation.clone());
        chart.chart.annotations = chart.annotations.clone();
        chart.selected_annotation = Some(annotation.id);
        chart.next_annotation_id = 4;
    }
    let payload = drawing_payload(serde_json::json!([{
        "operation": "remove",
        "chart_id": 7,
        "drawing_id": 3
    }]));

    let (result, _task) = terminal.handle_agent_host_action(&payload);
    let result: serde_json::Value = serde_json::from_str(&result).expect("result");
    let chart = &terminal.charts[&7];

    assert_eq!(result["success"], true);
    assert_eq!(result["operations"][0]["outcome"], "removed");
    assert!(chart.annotations.is_empty());
    assert!(chart.chart.annotations.is_empty());
    assert_eq!(chart.selected_annotation, None);
}

#[test]
fn drawing_action_requires_the_matching_running_tool() {
    let mut terminal = terminal_with_running_tool();
    let payload = drawing_payload(serde_json::json!([{
        "operation": "add",
        "chart_id": 7,
        "drawing": { "type": "horizontal_level", "price": 60_000.0 }
    }]));

    let (result, _task) = terminal.handle_agent_host_action(&payload);
    let result: serde_json::Value = serde_json::from_str(&result).expect("result");

    assert_eq!(result["success"], false);
    assert_eq!(result["action"], "manage_chart_drawings");
    assert_eq!(result["error"]["code"], "inactive_tool_call");
    assert!(terminal.charts[&7].annotations.is_empty());
}

#[test]
fn invalid_drawing_label_does_not_mutate_the_chart() {
    let mut terminal = terminal_with_running_drawing_tool();
    let payload = drawing_payload(serde_json::json!([{
        "operation": "add",
        "chart_id": 7,
        "drawing": {
            "type": "horizontal_level",
            "price": 60_000.0,
            "style": { "label": "unsafe\nlabel" }
        }
    }]));

    let (result, _task) = terminal.handle_agent_host_action(&payload);
    let result: serde_json::Value = serde_json::from_str(&result).expect("result");

    assert_eq!(result["success"], false);
    assert_eq!(result["error"]["code"], "invalid_drawing");
    assert!(terminal.charts[&7].annotations.is_empty());
}
