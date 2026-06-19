use super::*;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn hydromancer_read_remaining_decreases_then_saturates_at_zero() {
    let window = Duration::from_secs(HYDROMANCER_READ_TIMEOUT_SECS);
    assert_eq!(hydromancer_read_remaining(Duration::ZERO), window);
    assert_eq!(
        hydromancer_read_remaining(Duration::from_secs(10)),
        window - Duration::from_secs(10)
    );
    assert_eq!(
        hydromancer_read_remaining(Duration::from_secs(HYDROMANCER_READ_TIMEOUT_SECS)),
        Duration::ZERO
    );
    assert_eq!(
        hydromancer_read_remaining(Duration::from_secs(HYDROMANCER_READ_TIMEOUT_SECS * 4)),
        Duration::ZERO
    );
}

#[test]
fn hydromancer_connect_timeout_is_bounded_below_read_timeout() {
    assert_eq!(HYDROMANCER_CONNECT_TIMEOUT_SECS, 10);
    const _: () = assert!(HYDROMANCER_CONNECT_TIMEOUT_SECS < HYDROMANCER_READ_TIMEOUT_SECS);
}

#[test]
fn hydromancer_window_is_larger_than_the_app_level_stale_label() {
    // The status pane flags the feed as "Stale" after 75s; the reconnect
    // watchdog needs to fire later than that so the user sees the warning
    // before the connection is torn down.
    const APP_STALE_LABEL_SECS: u64 = 75;
    const _: () = assert!(HYDROMANCER_READ_TIMEOUT_SECS > APP_STALE_LABEL_SECS);
}

#[test]
fn hydromancer_manager_id_uses_key_and_generation() {
    let first = HydromancerStreamKey::new("hydro-secret-token-a", 7);
    let same = HydromancerStreamKey::new("hydro-secret-token-a", 7);
    let rotated_same_generation = HydromancerStreamKey::new("hydro-secret-token-b", 7);
    let next_generation = HydromancerStreamKey::new("hydro-secret-token-b", 8);

    assert_eq!(first.manager_id(), same.manager_id());
    assert_ne!(first.manager_id(), rotated_same_generation.manager_id());
    assert_ne!(first.manager_id(), next_generation.manager_id());
}

fn remove_manager_for_test(manager_id: u64) {
    let managers = HYDROMANCER_MANAGERS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let mut managers = managers.lock().unwrap_or_else(|e| e.into_inner());
    managers.remove(&manager_id);
}

fn insert_manager_for_test(
    manager_id: u64,
    task_id: u64,
    cmd_tx: mpsc::UnboundedSender<HydromancerCommand>,
) {
    let (_msg_tx, msg_rx) = broadcast::channel(1);
    let cmd_tx = HydromancerCommandSender::new_for_test(cmd_tx);
    let managers = HYDROMANCER_MANAGERS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let mut managers = managers.lock().unwrap_or_else(|e| e.into_inner());
    managers.insert(
        manager_id,
        HydromancerManager {
            task_id,
            cmd_tx,
            msg_rx,
        },
    );
}

fn manager_exists_for_test(manager_id: u64) -> bool {
    let managers = HYDROMANCER_MANAGERS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let managers = managers.lock().unwrap_or_else(|e| e.into_inner());
    managers.contains_key(&manager_id)
}

#[test]
fn finished_manager_cleanup_removes_matching_closed_entry() {
    let manager_id = u64::MAX - 100;
    let task_id = 1001;
    remove_manager_for_test(manager_id);

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    drop(cmd_rx);
    insert_manager_for_test(manager_id, task_id, cmd_tx);

    assert!(remove_hydromancer_manager_if_finished(manager_id, task_id));
    assert!(!manager_exists_for_test(manager_id));
}

#[test]
fn finished_manager_cleanup_keeps_replacement_entry() {
    let manager_id = u64::MAX - 101;
    let old_task_id = 2001;
    let replacement_task_id = 2002;
    remove_manager_for_test(manager_id);

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    insert_manager_for_test(manager_id, replacement_task_id, cmd_tx);

    assert!(!remove_hydromancer_manager_if_finished(
        manager_id,
        old_task_id
    ));
    assert!(manager_exists_for_test(manager_id));

    drop(cmd_rx);
    assert!(remove_hydromancer_manager_if_finished(
        manager_id,
        replacement_task_id
    ));
}

#[test]
fn reconnect_prunes_closed_registry_entry() {
    let stream_key = HydromancerStreamKey::new("hydro-secret-token-a", u64::MAX - 102);
    let manager_id = stream_key.manager_id();
    remove_manager_for_test(manager_id);

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    drop(cmd_rx);
    insert_manager_for_test(manager_id, 3001, cmd_tx);

    reconnect_hydromancer(stream_key);

    assert!(!manager_exists_for_test(manager_id));
}

#[test]
fn direct_reconnect_requests_are_coalesced_until_dequeued() {
    let stream_key = HydromancerStreamKey::new("hydro-secret-token-a", u64::MAX - 103);
    let manager_id = stream_key.manager_id();
    remove_manager_for_test(manager_id);

    let (raw_cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
    let command_sender = HydromancerCommandSender::new_for_test(raw_cmd_tx);
    let (_msg_tx, msg_rx) = broadcast::channel(1);
    {
        let managers = HYDROMANCER_MANAGERS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
        let mut managers = managers.lock().unwrap_or_else(|e| e.into_inner());
        managers.insert(
            manager_id,
            HydromancerManager {
                task_id: 4001,
                cmd_tx: command_sender.clone(),
                msg_rx,
            },
        );
    }

    reconnect_hydromancer(stream_key.clone());
    reconnect_hydromancer(stream_key.clone());

    let command = cmd_rx.try_recv().expect("first reconnect command");
    assert!(matches!(command, HydromancerCommand::Reconnect));
    assert!(cmd_rx.try_recv().is_err());

    command_sender.note_command_dequeued_for_test(&command);
    reconnect_hydromancer(stream_key);

    assert!(matches!(
        cmd_rx.try_recv().unwrap(),
        HydromancerCommand::Reconnect
    ));
    remove_manager_for_test(manager_id);
}

#[test]
fn routed_message_debug_redacts_json_payload() {
    let message = HydromancerRoutedMessage {
        msg_type: "connected".to_string(),
        data: Arc::new(serde_json::json!({
            "sessionId": "session-secret",
            "cursor": "cursor-secret"
        })),
    };

    let rendered = format!("{message:?}");

    assert!(rendered.contains("connected"));
    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains("session-secret"));
    assert!(!rendered.contains("cursor-secret"));
}

#[test]
fn command_debug_redacts_tracked_trade_addresses_and_payload() {
    let address = "0xabc0000000000000000000000000000000000000";
    let command = HydromancerCommand::Subscribe {
        topic: format!("userFills:{address}"),
        payload: serde_json::json!({
            "type": "subscribe",
            "subscription": {
                "type": "userFills",
                "addresses": [address],
                "token": "payload-token"
            }
        }),
    };

    let rendered = format!("{command:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(rendered.contains("subscription_type: Some(\"userFills\")"));
    assert!(!rendered.contains(address));
    assert!(!rendered.contains("payload-token"));
}
