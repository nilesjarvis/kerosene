use super::*;

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
    assert_eq!(next_text_msg_or_panic(&mut sent), payload.to_string());

    let routed = routed_msg_or_panic(&mut msg_rx);
    assert_eq!(routed.msg_type, "connected");
    assert_eq!(routed.data["sessionId"], "s1");
}
