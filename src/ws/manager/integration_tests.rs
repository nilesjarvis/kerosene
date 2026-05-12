//! Integration tests that drive `ws_manager_task` against a localhost
//! `tokio_tungstenite` server fixture. These exercise the loop the unit
//! tests can't reach: subscribe payloads landing on the wire, server data
//! reaching the broadcast layer, and reconnect-replay after a server-side
//! close.

use super::*;
use futures::StreamExt as _;
use serde_json::Value;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message as WsMsg;

#[derive(Debug)]
enum ServerEvent {
    Received(String),
    Connected,
    ClientClosed,
}

/// One-connection mock that echoes nothing; it just streams every received
/// frame back to the test thread via `events_tx` and lets the test script
/// outbound frames via `outbound_rx`. After the configured `Lifecycle`
/// expires, the connection is closed and the server stops accepting.
struct MockServer {
    addr: std::net::SocketAddr,
}

impl MockServer {
    fn url(&self) -> String {
        format!("ws://{}", self.addr)
    }
}

/// Drive a single mock-server connection. The accept loop runs once per
/// `accept_count`, allowing tests to verify reconnect-replay behavior. The
/// outbound channel stays open across iterations; the test closes the
/// current connection by sending on `kill_rx`.
async fn run_mock_server(
    listener: TcpListener,
    events_tx: mpsc::UnboundedSender<ServerEvent>,
    mut outbound_rx: mpsc::UnboundedReceiver<Value>,
    mut kill_rx: mpsc::UnboundedReceiver<()>,
    accept_count: usize,
) {
    for _ in 0..accept_count {
        let Ok((stream, _)) = listener.accept().await else {
            break;
        };
        let Ok(ws) = accept_async(stream).await else {
            break;
        };
        let _ = events_tx.send(ServerEvent::Connected);
        let (mut write, mut read) = ws.split();

        loop {
            tokio::select! {
                msg = read.next() => match msg {
                    Some(Ok(WsMsg::Text(text))) => {
                        let _ = events_tx.send(ServerEvent::Received(text.to_string()));
                    }
                    Some(Ok(WsMsg::Close(_))) | Some(Err(_)) | None => {
                        let _ = events_tx.send(ServerEvent::ClientClosed);
                        break;
                    }
                    _ => {}
                },
                outbound = outbound_rx.recv() => {
                    if let Some(payload) = outbound
                        && write.send(WsMsg::Text(payload.to_string().into())).await.is_err()
                    {
                        break;
                    }
                },
                _ = kill_rx.recv() => {
                    let _ = write.send(WsMsg::Close(None)).await;
                    break;
                }
            }
        }
    }
}

struct MockServerHandles {
    server: MockServer,
    events_rx: mpsc::UnboundedReceiver<ServerEvent>,
    outbound_tx: mpsc::UnboundedSender<Value>,
    kill_tx: mpsc::UnboundedSender<()>,
    _join: tokio::task::JoinHandle<()>,
}

async fn start_mock_server(accept_count: usize) -> MockServerHandles {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (events_tx, events_rx) = mpsc::unbounded_channel();
    let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
    let (kill_tx, kill_rx) = mpsc::unbounded_channel();
    let join = tokio::spawn(run_mock_server(
        listener,
        events_tx,
        outbound_rx,
        kill_rx,
        accept_count,
    ));
    MockServerHandles {
        server: MockServer { addr },
        events_rx,
        outbound_tx,
        kill_tx,
        _join: join,
    }
}

async fn wait_for_event<F>(
    rx: &mut mpsc::UnboundedReceiver<ServerEvent>,
    matcher: F,
    timeout: Duration,
) -> Option<ServerEvent>
where
    F: Fn(&ServerEvent) -> bool,
{
    tokio::time::timeout(timeout, async {
        while let Some(event) = rx.recv().await {
            if matcher(&event) {
                return Some(event);
            }
        }
        None
    })
    .await
    .ok()
    .flatten()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subscribe_payload_reaches_the_server() {
    let mut handles = start_mock_server(1).await;
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (msg_tx, _msg_rx) = broadcast::channel::<WsRoutedMessage>(64);

    let manager = tokio::spawn(ws_manager_task(handles.server.url(), cmd_rx, msg_tx));

    cmd_tx
        .send(WsCommand::Subscribe {
            topic: "l2Book.BTC".to_string(),
            payload: serde_json::json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": "BTC" },
            }),
        })
        .unwrap();

    let received = wait_for_event(
        &mut handles.events_rx,
        |e| matches!(e, ServerEvent::Received(_)),
        Duration::from_secs(2),
    )
    .await
    .expect("server should observe the subscribe payload");

    let ServerEvent::Received(text) = received else {
        unreachable!()
    };
    let payload: Value = serde_json::from_str(&text).expect("payload is JSON");
    assert_eq!(payload["method"], "subscribe");
    assert_eq!(payload["subscription"]["type"], "l2Book");
    assert_eq!(payload["subscription"]["coin"], "BTC");

    drop(cmd_tx);
    let _ = tokio::time::timeout(Duration::from_secs(1), manager).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_data_frames_reach_the_broadcast_receiver() {
    let mut handles = start_mock_server(1).await;
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (msg_tx, mut msg_rx) = broadcast::channel::<WsRoutedMessage>(64);

    let manager = tokio::spawn(ws_manager_task(handles.server.url(), cmd_rx, msg_tx));

    // Subscribe so the manager has a reason to be in the read loop.
    cmd_tx
        .send(WsCommand::Subscribe {
            topic: "l2Book.BTC".to_string(),
            payload: serde_json::json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": "BTC" },
            }),
        })
        .unwrap();

    // Wait for the subscribe to land on the server side so the connection
    // is up before we push outbound.
    let _ = wait_for_event(
        &mut handles.events_rx,
        |e| matches!(e, ServerEvent::Received(_)),
        Duration::from_secs(2),
    )
    .await
    .expect("server should observe the subscribe payload");

    handles
        .outbound_tx
        .send(serde_json::json!({
            "channel": "l2Book",
            "data": { "coin": "BTC", "levels": [[], []] },
        }))
        .unwrap();

    let routed = tokio::time::timeout(Duration::from_secs(2), msg_rx.recv())
        .await
        .expect("broadcast should fire")
        .expect("broadcast should not be closed");

    assert_eq!(routed.channel, "l2Book");
    assert_eq!(
        routed.data.get("coin").and_then(|v| v.as_str()),
        Some("BTC")
    );

    drop(cmd_tx);
    let _ = tokio::time::timeout(Duration::from_secs(1), manager).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconnect_replays_active_subscriptions_on_a_fresh_connection() {
    let mut handles = start_mock_server(2).await;
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (msg_tx, _msg_rx) = broadcast::channel::<WsRoutedMessage>(64);

    let manager = tokio::spawn(ws_manager_task(handles.server.url(), cmd_rx, msg_tx));

    cmd_tx
        .send(WsCommand::Subscribe {
            topic: "l2Book.BTC".to_string(),
            payload: serde_json::json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": "BTC" },
            }),
        })
        .unwrap();

    // First connection: confirm the subscribe lands.
    let first_received = wait_for_event(
        &mut handles.events_rx,
        |e| matches!(e, ServerEvent::Received(_)),
        Duration::from_secs(2),
    )
    .await
    .expect("server should observe the subscribe payload on the first connection");

    let ServerEvent::Received(first_text) = first_received else {
        unreachable!()
    };
    assert!(first_text.contains("l2Book"));

    // Close the current connection from the server side. outbound_tx stays
    // alive across iterations so the second connection can still receive
    // outbound frames if needed.
    handles.kill_tx.send(()).expect("send kill");

    // Manager should reconnect (base backoff 1s) and replay its active
    // subscriptions on the fresh connection.
    let second_received = wait_for_event(
        &mut handles.events_rx,
        |e| matches!(e, ServerEvent::Received(_)),
        Duration::from_secs(15),
    )
    .await
    .expect("manager should reconnect and replay subscriptions");

    let ServerEvent::Received(second_text) = second_received else {
        unreachable!()
    };
    assert!(second_text.contains("l2Book"));

    drop(cmd_tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), manager).await;
}
