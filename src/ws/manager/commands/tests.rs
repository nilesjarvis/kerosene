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
        }
    );
    assert_eq!(
        handle_ws_command(
            &mut subscriptions,
            WsCommand::Subscribe {
                topic: "trades".to_string(),
                payload: json!({"method":"ignored"}),
            },
        ),
        WsCommandAction::none()
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
            },
        ),
        WsCommandAction::none()
    );
    assert_eq!(
        handle_ws_command(
            &mut subscriptions,
            WsCommand::Unsubscribe {
                topic: "trades".to_string(),
            },
        ),
        WsCommandAction {
            outbound_payload: Some(json!({"method":"unsubscribe"})),
            disconnect_on_send_error: true,
            mark_ping_start: false,
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
        }
    );
}
