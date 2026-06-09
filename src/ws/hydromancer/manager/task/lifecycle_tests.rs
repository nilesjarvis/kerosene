use super::*;
use serde_json::json;

fn subscribe_cmd(topic: &str) -> HydromancerCommand {
    HydromancerCommand::Subscribe {
        topic: topic.to_string(),
        payload: json!({ "type": "subscribe", "channel": topic }),
    }
}

#[test]
fn connected_session_cleanup_flushes_pending_and_balances_hydromancer_telemetry() {
    let before = crate::ws::telemetry_snapshot().hydromancer_open_connections;
    let (msg_tx, mut msg_rx) = broadcast::channel(8);
    let mut coalescer = HydromancerCoalescedSender::with_interval(msg_tx, Duration::from_secs(60));

    telemetry_on_hydromancer_connect();
    coalescer.submit_json(json!({
        "type": "l2Book",
        "data": { "coin": "BTC", "seq": 1 },
    }));
    coalescer.submit_json(json!({
        "type": "l2Book",
        "data": { "coin": "BTC", "seq": 2 },
    }));

    finish_connected_hydromancer_session(&mut coalescer);

    assert_eq!(
        msg_rx.try_recv().expect("initial book should emit").data["data"]["seq"],
        1
    );
    assert_eq!(
        msg_rx
            .try_recv()
            .expect("pending book should flush during cleanup")
            .data["data"]["seq"],
        2
    );
    assert_eq!(
        crate::ws::telemetry_snapshot().hydromancer_open_connections,
        before
    );
}

#[test]
fn drain_pending_shutdown_handles_queued_rotation_before_reconnect() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut active_subs = ActiveHydromancerSubscriptions::default();

    assert!(tx.send(subscribe_cmd("liquidations")).is_ok());
    assert!(tx.send(HydromancerCommand::Shutdown).is_ok());

    assert_eq!(
        drain_pending_hydromancer_shutdown(&mut rx, &mut active_subs),
        HydromancerTaskControlFlow::Shutdown
    );
}

#[tokio::test]
async fn reconnect_sleep_exits_promptly_on_shutdown() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    active_subs.subscribe(
        "liquidations".to_string(),
        json!({ "channel": "liquidations" }),
    );
    assert!(tx.send(HydromancerCommand::Shutdown).is_ok());

    let result = match tokio::time::timeout(
        Duration::from_millis(50),
        hydromancer_sleep_or_shutdown(&mut rx, &mut active_subs, Duration::from_secs(60)),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => panic!("shutdown should interrupt the retry sleep"),
    };

    assert_eq!(result, HydromancerTaskControlFlow::Shutdown);
}

#[tokio::test]
async fn reconnect_sleep_processes_unsubscribe_before_retrying_old_key() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    active_subs.subscribe(
        "liquidations".to_string(),
        json!({ "channel": "liquidations" }),
    );
    assert!(
        tx.send(HydromancerCommand::Unsubscribe {
            topic: "liquidations".to_string(),
        })
        .is_ok()
    );

    let result = match tokio::time::timeout(
        Duration::from_millis(50),
        hydromancer_sleep_or_shutdown(&mut rx, &mut active_subs, Duration::from_secs(60)),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => panic!("unsubscribe should interrupt the retry sleep"),
    };

    assert_eq!(result, HydromancerTaskControlFlow::Continue);
    assert!(active_subs.is_empty());
}
