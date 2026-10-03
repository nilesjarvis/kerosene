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
        &mut active_subs,
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
        &mut active_subs,
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
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    let mut session = HydromancerSessionState::default();

    let disconnected = block_on(handle_hydromancer_ws_message(
        WsMsg::Text(r#"{"type":"userFills","sessionId":"s3","cursor":"c3","data":[]}"#.into()),
        &mut active_subs,
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

fn priority_subscriptions(
    tx: broadcast::Sender<HydromancerRoutedMessage>,
) -> ActiveHydromancerSubscriptions {
    let mut subscriptions = ActiveHydromancerSubscriptions::new(tx);
    for index in 0..11 {
        subscriptions.subscribe(format!("candle:{index}:1m"), json!({
            "method":"subscribe", "subscription":{"type":"candle", "coin":index.to_string(), "interval":"1m"}
        }));
    }
    // Register priority feeds last, as when a liquidation pane is opened after charts.
    for (topic, subscription) in [
        (
            "second",
            json!({"type":"candle", "coin":"BTC", "interval":"1s"}),
        ),
        (
            "tracked",
            json!({"type":"userFills", "addresses":["test-address"]}),
        ),
        ("liq", json!({"type":"liquidationFills"})),
    ] {
        subscriptions.subscribe(
            topic.into(),
            json!({"type":"subscribe", "subscription":subscription}),
        );
    }
    subscriptions
}

#[test]
fn chart_limit_releases_capacity_without_disconnecting_liquidations_or_one_second_candles() {
    let (mut write, mut sent) = mpsc::unbounded();
    let (msg_tx, mut msg_rx) = broadcast::channel(32);
    let mut coalescer = HydromancerCoalescedSender::new(msg_tx.clone());
    let mut subscriptions = priority_subscriptions(msg_tx.clone());
    let mut session = HydromancerSessionState::default();
    let error = json!({"type":"error", "code":"subscription_limit_exceeded",
        "message":"Maximum candle subscriptions reached for API key",
        "subscription":{"type":"candle", "coin":"10", "interval":"1m"}});
    assert!(!block_on(handle_hydromancer_ws_message(
        WsMsg::Text(error.to_string().into()),
        &mut subscriptions,
        &mut session,
        &msg_tx,
        &mut coalescer,
        &mut write,
    )));
    for _ in 0..11 {
        let message: serde_json::Value =
            serde_json::from_str(&next_text_msg_or_panic(&mut sent)).expect("unsubscribe JSON");
        assert_eq!(message["type"], "unsubscribe");
        assert_eq!(message["subscription"]["type"], "candle");
        assert_eq!(message["subscription"]["interval"], "1m");
    }
    assert!(sent.next().now_or_never().is_none());
    assert_eq!(routed_msg_or_panic(&mut msg_rx).msg_type, "marketFallback");
    assert!(
        msg_rx.try_recv().is_err(),
        "market rejection must not become a feed disconnect"
    );

    // The required feeds retain their socket and keep receiving data.
    assert!(!block_on(handle_hydromancer_ws_message(
        WsMsg::Text(
            json!({"type":"liquidationFills", "data":[]})
                .to_string()
                .into()
        ),
        &mut subscriptions,
        &mut session,
        &msg_tx,
        &mut coalescer,
        &mut write,
    )));
    assert_eq!(
        routed_msg_or_panic(&mut msg_rx).msg_type,
        "liquidationFills"
    );

    // A reconnect replays only the priority subscriptions, in priority order.
    assert!(!block_on(handle_hydromancer_ws_message(
        WsMsg::Text(
            json!({"type":"reconnected", "sessionId":"test-session"})
                .to_string()
                .into()
        ),
        &mut subscriptions,
        &mut session,
        &msg_tx,
        &mut coalescer,
        &mut write,
    )));
    for kind in ["liquidationFills", "userFills", "candle"] {
        let message: serde_json::Value =
            serde_json::from_str(&next_text_msg_or_panic(&mut sent)).expect("subscribe JSON");
        assert_eq!(message["subscription"]["type"], kind);
        if kind == "candle" {
            assert_eq!(message["subscription"]["interval"], "1s");
        }
    }
    assert!(sent.next().now_or_never().is_none());
}

#[test]
fn unscoped_limit_frees_market_capacity_before_retrying_priority_feeds() {
    let (mut write, mut sent) = mpsc::unbounded();
    let (msg_tx, mut msg_rx) = broadcast::channel(32);
    let mut coalescer = HydromancerCoalescedSender::new(msg_tx.clone());
    let mut subscriptions = priority_subscriptions(msg_tx.clone());
    let mut session = HydromancerSessionState::default();
    assert!(block_on(handle_hydromancer_ws_message(
        WsMsg::Text(
            json!({"type":"error", "message":"too many subscriptions"})
                .to_string()
                .into()
        ),
        &mut subscriptions,
        &mut session,
        &msg_tx,
        &mut coalescer,
        &mut write,
    )));
    for _ in 0..11 {
        let message: serde_json::Value =
            serde_json::from_str(&next_text_msg_or_panic(&mut sent)).expect("unsubscribe JSON");
        assert_eq!(message["type"], "unsubscribe");
    }
    assert!(sent.next().now_or_never().is_none());
    assert_eq!(session.session_id(), None);
    assert_eq!(routed_msg_or_panic(&mut msg_rx).msg_type, "marketFallback");
    assert_eq!(routed_msg_or_panic(&mut msg_rx).msg_type, "reconnecting");
    assert!(!block_on(handle_hydromancer_ws_message(
        WsMsg::Text(
            json!({"type":"connected", "sessionId":"fresh-session"})
                .to_string()
                .into()
        ),
        &mut subscriptions,
        &mut session,
        &msg_tx,
        &mut coalescer,
        &mut write,
    )));
    for kind in ["liquidationFills", "userFills", "candle"] {
        let message: serde_json::Value =
            serde_json::from_str(&next_text_msg_or_panic(&mut sent)).expect("subscribe JSON");
        assert_eq!(message["type"], "subscribe");
        assert_eq!(message["subscription"]["type"], kind);
    }
    assert!(sent.next().now_or_never().is_none());
    assert_eq!(routed_msg_or_panic(&mut msg_rx).msg_type, "connected");
}

#[test]
fn capacity_close_discards_overloaded_resume_session_and_required_limit_errors_remain_visible() {
    use tokio_tungstenite::tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};
    let (mut write, _sent) = mpsc::unbounded();
    let (msg_tx, mut msg_rx) = broadcast::channel(32);
    let mut coalescer = HydromancerCoalescedSender::new(msg_tx.clone());
    let mut subscriptions = priority_subscriptions(msg_tx.clone());
    let mut session = HydromancerSessionState::default();
    session.apply_text_frame(
        &parse_hydromancer_text_frame(
            r#"{"type":"connected","sessionId":"old-session","cursor":"old-cursor"}"#,
        )
        .expect("connected frame"),
    );
    assert!(block_on(handle_hydromancer_ws_message(
        WsMsg::Close(Some(CloseFrame {
            code: CloseCode::Policy,
            reason: "connection limit exceeded".into()
        })),
        &mut subscriptions,
        &mut session,
        &msg_tx,
        &mut coalescer,
        &mut write,
    )));
    assert_eq!(session.session_id(), None);
    assert_eq!(session.last_cursor(), None);
    assert_eq!(subscriptions.payloads().count(), 3);
    assert_eq!(routed_msg_or_panic(&mut msg_rx).msg_type, "marketFallback");
    assert_eq!(routed_msg_or_panic(&mut msg_rx).msg_type, "reconnecting");

    assert!(!block_on(handle_hydromancer_ws_message(
        WsMsg::Text(json!({"type":"error", "message":"subscription limit reached", "subscription":{"type":"liquidationFills"}}).to_string().into()),
        &mut subscriptions, &mut session, &msg_tx, &mut coalescer, &mut write,
    )));
    assert_eq!(routed_msg_or_panic(&mut msg_rx).msg_type, "marketFallback");
    assert_eq!(routed_msg_or_panic(&mut msg_rx).msg_type, "error");
}

#[test]
fn market_only_overload_releases_the_unused_socket() {
    let (mut write, _sent) = mpsc::unbounded();
    let (msg_tx, _msg_rx) = broadcast::channel(8);
    let mut coalescer = HydromancerCoalescedSender::new(msg_tx.clone());
    let mut subscriptions = ActiveHydromancerSubscriptions::new(msg_tx.clone());
    subscriptions.subscribe(
        "book".into(),
        json!({"method":"subscribe", "subscription":{"type":"l2Book", "coin":"BTC"}}),
    );
    assert!(block_on(handle_hydromancer_ws_message(
        WsMsg::Text(json!({"type":"error", "code":"subscription_limit_exceeded", "subscription":{"type":"l2Book", "coin":"BTC"}}).to_string().into()),
        &mut subscriptions, &mut HydromancerSessionState::default(), &msg_tx, &mut coalescer, &mut write,
    )));
    assert!(subscriptions.is_empty());
}
