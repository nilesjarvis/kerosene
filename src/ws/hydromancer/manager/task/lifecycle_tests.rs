use super::*;
use serde_json::json;
use tokio::net::TcpListener;
use zeroize::Zeroizing;

#[derive(Debug)]
enum HangingServerEvent {
    Connected,
}

struct HangingHandshakeServer {
    url: String,
    events_rx: mpsc::UnboundedReceiver<HangingServerEvent>,
    shutdown_tx: mpsc::UnboundedSender<()>,
    _join: tokio::task::JoinHandle<()>,
}

fn subscribe_cmd(topic: &str) -> HydromancerCommand {
    HydromancerCommand::Subscribe {
        topic: topic.to_string(),
        payload: json!({ "type": "subscribe", "channel": topic }),
    }
}

async fn run_hanging_handshake_server(
    listener: TcpListener,
    events_tx: mpsc::UnboundedSender<HangingServerEvent>,
    mut shutdown_rx: mpsc::UnboundedReceiver<()>,
) {
    let mut held_streams = Vec::new();
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else {
                    break;
                };
                held_streams.push(stream);
                let _ = events_tx.send(HangingServerEvent::Connected);
            }
            _ = shutdown_rx.recv() => {
                break;
            }
        }
    }
}

async fn start_hanging_handshake_server() -> HangingHandshakeServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (events_tx, events_rx) = mpsc::unbounded_channel();
    let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
    let join = tokio::spawn(run_hanging_handshake_server(
        listener,
        events_tx,
        shutdown_rx,
    ));
    HangingHandshakeServer {
        url: format!("ws://{addr}"),
        events_rx,
        shutdown_tx,
        _join: join,
    }
}

async fn wait_for_hanging_server_connection(
    rx: &mut mpsc::UnboundedReceiver<HangingServerEvent>,
    timeout: Duration,
) -> Option<HangingServerEvent> {
    tokio::time::timeout(timeout, rx.recv())
        .await
        .ok()
        .flatten()
}

async fn wait_for_hydromancer_message<F>(
    rx: &mut broadcast::Receiver<HydromancerRoutedMessage>,
    matcher: F,
    timeout: Duration,
) -> Option<HydromancerRoutedMessage>
where
    F: Fn(&HydromancerRoutedMessage) -> bool,
{
    tokio::time::timeout(timeout, async {
        while let Ok(message) = rx.recv().await {
            if matcher(&message) {
                return Some(message);
            }
        }
        None
    })
    .await
    .ok()
    .flatten()
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

#[tokio::test]
async fn hanging_connect_emits_reconnecting_after_connect_timeout() {
    let mut server = start_hanging_handshake_server().await;
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (msg_tx, mut msg_rx) = broadcast::channel(16);

    let manager = tokio::spawn(hydromancer_manager_task_with_options(
        Zeroizing::new("hydro-secret".to_string()),
        cmd_rx,
        msg_tx,
        Duration::from_millis(50),
        Some(server.url.clone()),
    ));

    assert!(cmd_tx.send(subscribe_cmd("liquidations")).is_ok());

    wait_for_hanging_server_connection(&mut server.events_rx, Duration::from_secs(1))
        .await
        .expect("server should accept the stalled TCP connection");
    let reconnecting = wait_for_hydromancer_message(
        &mut msg_rx,
        |message| message.msg_type == "reconnecting",
        Duration::from_secs(1),
    )
    .await
    .expect("connect timeout should emit a reconnecting control message");

    let error = reconnecting.data["error"]
        .as_str()
        .expect("timeout reconnecting message should include an error");
    assert!(error.contains("connect timeout after"));
    assert!(!error.contains("hydro-secret"));
    assert_eq!(reconnecting.data["retryDelaySecs"], 1);

    let _ = cmd_tx.send(HydromancerCommand::Shutdown);
    let _ = server.shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(1), manager).await;
}

#[tokio::test]
async fn shutdown_cancels_hanging_connect_before_timeout() {
    let mut server = start_hanging_handshake_server().await;
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (msg_tx, _msg_rx) = broadcast::channel(16);

    let manager = tokio::spawn(hydromancer_manager_task_with_options(
        Zeroizing::new("hydro-secret".to_string()),
        cmd_rx,
        msg_tx,
        Duration::from_secs(60),
        Some(server.url.clone()),
    ));

    assert!(cmd_tx.send(subscribe_cmd("liquidations")).is_ok());
    wait_for_hanging_server_connection(&mut server.events_rx, Duration::from_secs(1))
        .await
        .expect("server should accept the stalled TCP connection");
    assert!(cmd_tx.send(HydromancerCommand::Shutdown).is_ok());

    tokio::time::timeout(Duration::from_millis(500), manager)
        .await
        .expect("shutdown should cancel the in-flight connect wrapper")
        .expect("manager task should not panic");
    let _ = server.shutdown_tx.send(());
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
            payload: json!({ "channel": "liquidations" }),
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

#[tokio::test]
async fn reconnect_sleep_returns_immediately_when_no_subscriptions_remain() {
    let (_tx, mut rx) = mpsc::unbounded_channel();
    let mut active_subs = ActiveHydromancerSubscriptions::default();

    let result = match tokio::time::timeout(
        Duration::from_millis(50),
        hydromancer_sleep_or_shutdown(&mut rx, &mut active_subs, Duration::from_secs(60)),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => panic!("empty subscription sleep should not keep an old-key task alive"),
    };

    assert_eq!(result, HydromancerTaskControlFlow::Continue);
}

#[tokio::test]
async fn idle_wait_shuts_down_after_timeout_without_subscriptions() {
    let (_tx, mut rx) = mpsc::unbounded_channel();
    let mut active_subs = ActiveHydromancerSubscriptions::default();

    let result = match tokio::time::timeout(
        Duration::from_millis(100),
        hydromancer_wait_for_subscription_or_shutdown(
            &mut rx,
            &mut active_subs,
            Duration::from_millis(10),
        ),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => panic!("idle manager should shut down after the idle timeout"),
    };

    assert_eq!(result, HydromancerTaskControlFlow::Shutdown);
    assert!(active_subs.is_empty());
}

#[tokio::test]
async fn idle_wait_accepts_subscription_before_timeout() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    assert!(tx.send(subscribe_cmd("liquidations")).is_ok());

    let result = match tokio::time::timeout(
        Duration::from_millis(100),
        hydromancer_wait_for_subscription_or_shutdown(
            &mut rx,
            &mut active_subs,
            Duration::from_secs(60),
        ),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => panic!("queued subscription should wake the idle manager"),
    };

    assert_eq!(result, HydromancerTaskControlFlow::Continue);
    assert!(!active_subs.is_empty());
}
