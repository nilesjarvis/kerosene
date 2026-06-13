use super::*;

#[test]
fn connected_frame_replays_active_subscription_payloads() {
    let (mut write, mut sent) = mpsc::unbounded();
    let (msg_tx, mut msg_rx) = broadcast::channel(8);
    let mut coalescer = HydromancerCoalescedSender::new(msg_tx.clone());
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
        &mut coalescer,
        &mut write,
    ));

    assert!(!disconnected);
    assert!(session.connection_ready());
    assert_eq!(next_text_msg_or_panic(&mut sent), payload.to_string());

    let routed = routed_msg_or_panic(&mut msg_rx);
    assert_eq!(routed.msg_type, "connected");
    assert!(routed.data.get("sessionId").is_none());
    assert!(routed.data.get("cursor").is_none());
}

#[test]
fn reconnected_frame_replays_active_subscription_payloads() {
    let (mut write, mut sent) = mpsc::unbounded();
    let (msg_tx, mut msg_rx) = broadcast::channel(8);
    let mut coalescer = HydromancerCoalescedSender::new(msg_tx.clone());
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    let mut session = HydromancerSessionState::default();
    let payload = json!({
        "type": "subscribe",
        "subscription": { "type": "userFills", "user": "0xabc" },
    });
    active_subs.subscribe("fills:0xabc".to_string(), payload.clone());

    let disconnected = block_on(handle_hydromancer_ws_message(
        WsMsg::Text(r#"{"type":"reconnected","sessionId":"s2"}"#.into()),
        &active_subs,
        &mut session,
        &msg_tx,
        &mut coalescer,
        &mut write,
    ));

    assert!(!disconnected);
    assert!(session.connection_ready());
    assert_eq!(next_text_msg_or_panic(&mut sent), payload.to_string());

    let routed = routed_msg_or_panic(&mut msg_rx);
    assert_eq!(routed.msg_type, "reconnected");
    assert!(routed.data.get("sessionId").is_none());
    assert!(routed.data.get("cursor").is_none());
}

#[test]
fn data_frame_strips_resume_material_before_routing() {
    let (mut write, _sent) = mpsc::unbounded();
    let (msg_tx, mut msg_rx) = broadcast::channel(8);
    let mut coalescer = HydromancerCoalescedSender::new(msg_tx.clone());
    let active_subs = ActiveHydromancerSubscriptions::default();
    let mut session = HydromancerSessionState::default();

    let disconnected = block_on(handle_hydromancer_ws_message(
        WsMsg::Text(r#"{"type":"userFills","sessionId":"s3","cursor":"c3","data":[]}"#.into()),
        &active_subs,
        &mut session,
        &msg_tx,
        &mut coalescer,
        &mut write,
    ));

    assert!(!disconnected);
    assert_eq!(session.last_cursor(), Some("c3"));

    let routed = routed_msg_or_panic(&mut msg_rx);
    assert_eq!(routed.msg_type, "userFills");
    assert_eq!(routed.data["data"], json!([]));
    assert!(routed.data.get("sessionId").is_none());
    assert!(routed.data.get("cursor").is_none());
}
