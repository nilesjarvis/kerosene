use super::*;

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
fn connected_subscribe_sends_payload_for_same_topic_different_payload() {
    let (mut write, mut sent) = mpsc::unbounded();
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    active_subs.subscribe(
        "fills".to_string(),
        json!({
            "type": "subscribe",
            "subscription": { "type": "userFills", "user": "0xabc" },
        }),
    );
    let mut session = HydromancerSessionState::default();
    let frame = parse_hydromancer_text_frame(r#"{"type":"connected","sessionId":"s1"}"#)
        .expect("connected frame");
    session.apply_text_frame(&frame);
    let second_payload = json!({
        "type": "subscribe",
        "subscription": { "type": "userFills", "user": "0xdef" },
    });

    let disconnected = block_on(handle_hydromancer_command(
        HydromancerCommand::Subscribe {
            topic: "fills".to_string(),
            payload: second_payload.clone(),
        },
        &mut active_subs,
        &session,
        &mut write,
    ));

    assert!(!disconnected);
    assert_eq!(
        next_text_msg_or_panic(&mut sent),
        second_payload.to_string()
    );
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
            payload: json!({
                "type": "subscribe",
                "subscription": { "type": "userFills", "user": "0xabc" },
            }),
        },
        &mut active_subs,
        &session,
        &mut write,
    ));

    assert!(disconnected);
    assert!(active_subs.is_empty());
    assert_eq!(
        next_text_msg_or_panic(&mut sent),
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
            payload: json!({
                "type": "subscribe",
                "subscription": { "type": "userFills", "user": "0xabc" },
            }),
        },
        &mut active_subs,
        &session,
        &mut write,
    ));

    assert!(disconnected);
    assert!(!active_subs.is_empty());
}
