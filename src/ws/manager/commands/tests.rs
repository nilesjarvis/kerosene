use serde_json::json;

use super::*;

#[test]
fn subscribe_command_sends_payload_for_first_reference_only() {
    let mut subscriptions = ActiveWsSubscriptions::default();

    assert_eq!(
        handle_ws_command(
            &mut subscriptions,
            WsCommand::Subscribe {
                topic: "trades".to_string(),
                payload: json!({"method":"subscribe"}),
            },
        ),
        WsCommandAction {
            outbound_payload: Some(json!({"method":"subscribe"})),
            disconnect_on_send_error: true,
            mark_ping_start: false,
            disconnect_after_handling: false,
        }
    );
    assert_eq!(
        handle_ws_command(
            &mut subscriptions,
            WsCommand::Subscribe {
                topic: "trades".to_string(),
                payload: json!({"method":"subscribe"}),
            },
        ),
        WsCommandAction::none()
    );
}

#[test]
fn subscribe_command_sends_payload_for_same_topic_different_payload() {
    let mut subscriptions = ActiveWsSubscriptions::default();
    subscriptions.subscribe("trades".to_string(), json!({"method":"subscribe"}));

    assert_eq!(
        handle_ws_command(
            &mut subscriptions,
            WsCommand::Subscribe {
                topic: "trades".to_string(),
                payload: json!({"method":"different"}),
            },
        ),
        WsCommandAction {
            outbound_payload: Some(json!({"method":"different"})),
            disconnect_on_send_error: true,
            mark_ping_start: false,
            disconnect_after_handling: false,
        }
    );
}

#[test]
fn unsubscribe_command_sends_payload_only_for_final_reference() {
    let mut subscriptions = ActiveWsSubscriptions::default();
    subscriptions.subscribe("trades".to_string(), json!({"method":"subscribe"}));
    subscriptions.subscribe("trades".to_string(), json!({"method":"subscribe"}));

    assert_eq!(
        handle_ws_command(
            &mut subscriptions,
            WsCommand::Unsubscribe {
                topic: "trades".to_string(),
                payload: json!({"method":"subscribe"}),
            },
        ),
        WsCommandAction::none()
    );
    assert_eq!(
        handle_ws_command(
            &mut subscriptions,
            WsCommand::Unsubscribe {
                topic: "trades".to_string(),
                payload: json!({"method":"subscribe"}),
            },
        ),
        WsCommandAction {
            outbound_payload: Some(json!({"method":"unsubscribe"})),
            disconnect_on_send_error: true,
            mark_ping_start: false,
            disconnect_after_handling: false,
        }
    );
}

#[test]
fn unsubscribe_missing_topic_is_noop() {
    let mut subscriptions = ActiveWsSubscriptions::default();

    assert_eq!(
        handle_ws_command(
            &mut subscriptions,
            WsCommand::Unsubscribe {
                topic: "missing".to_string(),
                payload: json!({"method":"subscribe"}),
            },
        ),
        WsCommandAction::none()
    );
}

#[test]
fn ping_command_sends_ping_and_marks_latency_start() {
    let mut subscriptions = ActiveWsSubscriptions::default();

    assert_eq!(
        handle_ws_command(&mut subscriptions, WsCommand::Ping),
        WsCommandAction {
            outbound_payload: Some(json!({"method":"ping"})),
            disconnect_on_send_error: true,
            mark_ping_start: true,
            disconnect_after_handling: false,
        }
    );
}

#[test]
fn reconnect_command_disconnects_without_changing_subscriptions() {
    let mut subscriptions = ActiveWsSubscriptions::default();
    subscriptions.subscribe("trades".to_string(), json!({"method":"subscribe"}));

    assert_eq!(
        handle_ws_command(&mut subscriptions, WsCommand::Reconnect),
        WsCommandAction {
            outbound_payload: None,
            disconnect_on_send_error: false,
            mark_ping_start: false,
            disconnect_after_handling: true,
        }
    );
    assert!(subscriptions.payloads().next().is_some());
}
