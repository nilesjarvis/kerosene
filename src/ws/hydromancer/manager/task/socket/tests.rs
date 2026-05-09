use super::*;

use futures::channel::mpsc;
use futures::{FutureExt as _, StreamExt as _, executor::block_on};
use serde_json::json;

fn text_msg(msg: WsMsg) -> String {
    match msg {
        WsMsg::Text(text) => text.to_string(),
        other => panic!("expected text message, got {other:?}"),
    }
}

#[test]
fn subscribe_before_connection_ready_records_without_sending() {
    let (mut write, mut sent) = mpsc::unbounded();
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    let session = HydromancerSessionState::default();
    let payload = json!({
        "type": "subscribe",
        "subscription": { "type": "userFills", "user": "0xabc" },
    });

    let disconnected = block_on(handle_hydromancer_command(
        HydromancerCommand::Subscribe {
            topic: "fills:0xabc".to_string(),
            payload,
        },
        &mut active_subs,
        &session,
        &mut write,
    ));

    assert!(!disconnected);
    assert!(!active_subs.is_empty());
    assert!(sent.next().now_or_never().is_none());
}

#[test]
fn connected_frame_replays_active_subscription_payloads() {
    let (mut write, mut sent) = mpsc::unbounded();
    let (msg_tx, mut msg_rx) = broadcast::channel(8);
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    let mut session = HydromancerSessionState::default();
    let payload = json!({
        "type": "subscribe",
        "subscription": { "type": "userFills", "user": "0xabc" },
    });
    active_subs.subscribe("fills:0xabc".to_string(), payload.clone());

    let disconnected = block_on(handle_hydromancer_ws_message(
        WsMsg::Text(r#"{"type":"connected","sessionId":"s1","cursor":"c1"}"#.into()),
        &active_subs,
        &mut session,
        &msg_tx,
        &mut write,
    ));

    assert!(!disconnected);
    assert!(session.connection_ready());
    assert_eq!(
        text_msg(block_on(sent.next()).unwrap()),
        payload.to_string()
    );

    let routed = msg_rx.try_recv().expect("connected frame should broadcast");
    assert_eq!(routed.msg_type, "connected");
    assert_eq!(routed.data["sessionId"], "s1");
}

#[test]
fn final_unsubscribe_sends_unsubscribe_payload_and_disconnects() {
    let (mut write, mut sent) = mpsc::unbounded();
    let session = HydromancerSessionState::default();
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    active_subs.subscribe(
        "fills:0xabc".to_string(),
        json!({
            "type": "subscribe",
            "subscription": { "type": "userFills", "user": "0xabc" },
        }),
    );

    let disconnected = block_on(handle_hydromancer_command(
        HydromancerCommand::Unsubscribe {
            topic: "fills:0xabc".to_string(),
        },
        &mut active_subs,
        &session,
        &mut write,
    ));

    assert!(disconnected);
    assert!(active_subs.is_empty());
    assert_eq!(
        text_msg(block_on(sent.next()).unwrap()),
        json!({
            "type": "unsubscribe",
            "subscription": { "type": "userFills", "user": "0xabc" },
        })
        .to_string()
    );
}

#[test]
fn unsubscribe_send_failure_disconnects_even_with_remaining_subscriptions() {
    let (mut write, sent) = mpsc::unbounded();
    drop(sent);
    let session = HydromancerSessionState::default();
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    active_subs.subscribe(
        "fills:0xabc".to_string(),
        json!({
            "type": "subscribe",
            "subscription": { "type": "userFills", "user": "0xabc" },
        }),
    );
    active_subs.subscribe(
        "fills:0xdef".to_string(),
        json!({
            "type": "subscribe",
            "subscription": { "type": "userFills", "user": "0xdef" },
        }),
    );

    let disconnected = block_on(handle_hydromancer_command(
        HydromancerCommand::Unsubscribe {
            topic: "fills:0xabc".to_string(),
        },
        &mut active_subs,
        &session,
        &mut write,
    ));

    assert!(disconnected);
    assert!(!active_subs.is_empty());
}
