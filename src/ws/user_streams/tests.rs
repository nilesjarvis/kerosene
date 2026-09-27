use super::*;
use crate::ws::WsUserData;
use std::{cell::Cell, future::ready, task::Poll};

#[tokio::test]
async fn action_emission_preserves_delivery_failure_reconnect_order_and_pause() {
    const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
    for kind in 0..3 {
        for receiver_closed in [false, true] {
            for send_ok in [false, true] {
                for pause in [Duration::ZERO, Duration::from_secs(3600)] {
                    let (raw_cmd_tx, cmd_rx) = mpsc::unbounded_channel();
                    let cmd_tx = WsCommandSender::new_for_test(raw_cmd_tx);
                    let mut cmd_rx = if receiver_closed {
                        drop(cmd_rx);
                        None
                    } else {
                        Some(cmd_rx)
                    };
                    let update = (Some(ADDRESS.to_string()), WsUserData::Lagged { skipped: 9 });
                    let action = match kind {
                        0 => UserStreamReceiveAction::Emit(update),
                        1 => UserStreamReceiveAction::EmitAndReconnect(update),
                        _ => UserStreamReceiveAction::Ignore,
                    };
                    let emitted = Cell::new(false);
                    let emission = action.emit(
                        &cmd_tx,
                        |(source, update)| {
                            emitted.set(true);
                            assert_eq!(source.as_deref(), Some(ADDRESS));
                            assert!(matches!(update, WsUserData::Lagged { skipped: 9 }));
                            if kind == 1 {
                                assert!(
                                    matches!(
                                        cmd_rx.as_mut().expect("open receiver").try_recv(),
                                        Ok(WsCommand::Reconnect)
                                    ),
                                    "reconnect must be queued before emitting"
                                );
                            }
                            ready(send_ok)
                        },
                        pause,
                    );
                    {
                        let mut emission = std::pin::pin!(emission);
                        let result = futures::poll!(emission.as_mut());
                        let expected = match kind {
                            0 => Poll::Ready(send_ok),
                            1 if receiver_closed || !send_ok => Poll::Ready(false),
                            1 if !pause.is_zero() => Poll::Pending,
                            _ => Poll::Ready(true),
                        };
                        assert_eq!(result, expected);
                    }
                    assert_eq!(emitted.get(), kind == 0 || (kind == 1 && !receiver_closed));
                    if let Some(receiver) = cmd_rx.as_mut() {
                        assert!(receiver.try_recv().is_err(), "no extra reconnects");
                    }
                }
            }
        }
    }
}

#[test]
fn opted_out_stream_ignores_broadcast_all_mids_frames() {
    let update = parse_user_stream_routed_message(
        "allMids",
        &serde_json::json!({ "mids": { "BTC": "100" } }),
        Some("0xabc0000000000000000000000000000000000000"),
        Some("0xabc0000000000000000000000000000000000000".to_string()),
        false,
    );

    assert!(update.is_none());
}

#[test]
fn stream_params_debug_redacts_address() {
    const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

    let params = WsUserDataStreamParams::without_mids(
        Some(ADDRESS.to_string()),
        vec!["".to_string(), "dex-a".to_string()],
    );
    let rendered = format!("{params:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains(ADDRESS), "{rendered}");
    assert!(rendered.contains("dex-a"), "{rendered}");
    assert!(rendered.contains("include_mids: false"), "{rendered}");
}

#[test]
fn opted_in_stream_keeps_all_mids_frames() {
    let Some((source_addr, WsUserData::AllMids(mids))) = parse_user_stream_routed_message(
        "allMids",
        &serde_json::json!({ "mids": { "BTC": "100" } }),
        Some("0xabc0000000000000000000000000000000000000"),
        Some("0xabc0000000000000000000000000000000000000".to_string()),
        true,
    ) else {
        panic!("expected mids update");
    };

    assert_eq!(
        source_addr.as_deref(),
        Some("0xabc0000000000000000000000000000000000000")
    );
    assert_eq!(mids.get("BTC"), Some(&100.0));
}

#[test]
fn normal_user_data_action_does_not_request_reconnect() {
    let action = user_stream_routed_action(
        "allMids",
        &serde_json::json!({ "mids": { "BTC": "100" } }),
        Some("0xabc0000000000000000000000000000000000000"),
        Some("0xabc0000000000000000000000000000000000000".to_string()),
        true,
    );

    assert!(!action.should_reconnect_after_emit());
    let UserStreamReceiveAction::Emit((source_addr, WsUserData::AllMids(mids))) = action else {
        panic!("expected normal mids update");
    };
    assert_eq!(
        source_addr.as_deref(),
        Some("0xabc0000000000000000000000000000000000000")
    );
    assert_eq!(mids.get("BTC"), Some(&100.0));
}

#[test]
fn lagged_user_data_action_requests_reconnect() {
    let action = user_stream_lagged_action(
        Some("0xabc0000000000000000000000000000000000000".to_string()),
        7,
    );

    assert!(action.should_reconnect_after_emit());
    let UserStreamReceiveAction::EmitAndReconnect((source_addr, WsUserData::Lagged { skipped })) =
        action
    else {
        panic!("expected lagged reconnect update");
    };
    assert_eq!(
        source_addr.as_deref(),
        Some("0xabc0000000000000000000000000000000000000")
    );
    assert_eq!(skipped, 7);
}

#[test]
fn malformed_targeted_spot_state_forces_reconciliation_and_reconnect() {
    const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
    let action = user_stream_routed_action(
        "spotState",
        &serde_json::json!({
            "user": ADDRESS,
            "spotState": { "balances": "invalid" }
        }),
        Some(ADDRESS),
        Some(ADDRESS.to_string()),
        true,
    );

    assert!(action.should_reconnect_after_emit());
    let UserStreamReceiveAction::EmitAndReconnect((source_addr, WsUserData::Lagged { skipped: 1 })) =
        action
    else {
        panic!("malformed targeted spotState must reconcile");
    };
    assert_eq!(source_addr.as_deref(), Some(ADDRESS));
}

#[test]
fn malformed_spot_state_for_another_address_is_ignored() {
    const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
    const OTHER: &str = "0xdef0000000000000000000000000000000000000";
    let action = user_stream_routed_action(
        "spotState",
        &serde_json::json!({
            "user": OTHER,
            "spotState": { "balances": "invalid" }
        }),
        Some(ADDRESS),
        Some(ADDRESS.to_string()),
        true,
    );

    assert!(matches!(action, UserStreamReceiveAction::Ignore));
}

#[test]
fn user_data_lag_requests_shared_ws_reconnect() {
    let (raw_cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
    let cmd_tx = WsCommandSender::new_for_test(raw_cmd_tx);

    assert!(cmd_tx.request_lag_reconnect());
    assert!(matches!(cmd_rx.try_recv().unwrap(), WsCommand::Reconnect));
}

#[tokio::test]
async fn lag_emit_requests_reconnect_before_downstream_send_failure() {
    let (raw_cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
    let cmd_tx = WsCommandSender::new_for_test(raw_cmd_tx);

    let emitted = emit_after_reconnect(
        || cmd_tx.request_lag_reconnect(),
        (None::<String>, WsUserData::Lagged { skipped: 7 }),
        |_update| async { false },
        Duration::ZERO,
    )
    .await;

    assert!(!emitted);
    assert!(matches!(cmd_rx.try_recv().unwrap(), WsCommand::Reconnect));
}
