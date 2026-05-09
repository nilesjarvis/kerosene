use super::*;
use crate::ws::HydromancerWsMessage;

#[test]
fn user_fills_parser_accepts_live_multi_address_payload() {
    let payload = serde_json::json!({
        "type": "userFills",
        "fills": [[
            "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            {
                "coin": "HYPE",
                "px": "10.5",
                "sz": "2.25",
                "side": "B",
                "time": 1710000000000_u64,
                "startPosition": "0",
                "dir": "Open Long",
                "closedPnl": "1.23",
                "fee": "0.01",
                "feeToken": "USDC",
                "tid": 42_u64,
                "hash": "0xabc",
                "oid": 7_u64,
                "txIndex": 3_u64
            }
        ]]
    });

    let fills = hydromancer_fill_items(&payload, "userFills").expect("live fills");
    let trade = parse_tracked_trade_event(&fills[0]).expect("tracked trade");

    assert_eq!(trade.address, "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    assert_eq!(trade.coin, "HYPE");
    assert_eq!(trade.price, 10.5);
    assert_eq!(trade.size, 2.25);
    assert!(trade.is_buy);
    assert_eq!(trade.start_position, Some(0.0));
    assert_eq!(trade.closed_pnl, 1.23);
    assert_eq!(trade.fee_token, "USDC");
    assert_eq!(trade.tid, Some(42));
    assert_eq!(trade.oid, Some(7));
    assert_eq!(trade.tx_index, 3);
}

#[test]
fn user_fills_parser_rejects_malformed_numeric_fields() {
    let fill = serde_json::json!([
        "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        {
            "coin": "HYPE",
            "px": "bad",
            "sz": "2.25",
            "side": "B",
            "time": 1710000000000_u64,
            "startPosition": "NaN",
            "dir": "Open Long",
            "closedPnl": "1.23",
            "fee": "0.01",
            "feeToken": "USDC",
            "txIndex": 3_u64
        }
    ]);

    assert!(parse_tracked_trade_event(&fill).is_none());
}

#[test]
fn liquidation_parser_rejects_malformed_numeric_fields() {
    let fill = serde_json::json!([
        "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        {
            "coin": "HYPE",
            "px": "10.5",
            "sz": "inf",
            "side": "A",
            "time": 1710000000000_u64,
            "txIndex": 3_u64,
            "liquidation": {
                "method": "market",
                "liquidatedUser": "0xabc"
            }
        }
    ]);

    assert!(parse_liquidation_event(&fill).is_none());
}

#[test]
fn fill_item_parser_only_accepts_matching_replay_channel() {
    let payload = serde_json::json!({
        "type": "replay",
        "channel": "userFills",
        "data": []
    });

    assert!(hydromancer_fill_items(&payload, "userFills").is_some());
    assert!(hydromancer_fill_items(&payload, "liquidationFills").is_none());
}

#[test]
fn control_message_uses_hydromancer_message_field_for_errors() {
    let data = serde_json::json!({
        "type": "error",
        "message": "Too many subscriptions"
    });

    let Some(HydromancerWsMessage::Disconnected(error)) =
        hydromancer_control_message("error", &data)
    else {
        panic!("expected disconnected control message");
    };

    assert_eq!(error, "Too many subscriptions");
}

#[test]
fn control_message_reports_reconnect_delay() {
    let data = serde_json::json!({
        "error": "heartbeat timeout after 95s",
        "retryDelaySecs": 2_u64,
    });

    let Some(HydromancerWsMessage::Reconnecting {
        error,
        retry_delay_secs,
    }) = hydromancer_control_message("reconnecting", &data)
    else {
        panic!("expected reconnecting control message");
    };

    assert_eq!(
        error,
        "Hydromancer network timeout: heartbeat timeout after 95s"
    );
    assert_eq!(retry_delay_secs, 2);
}

#[test]
fn control_message_labels_authentication_failures() {
    let data = serde_json::json!({
        "error": "HTTP 401 Unauthorized",
        "retryDelaySecs": 2_u64,
    });

    let Some(HydromancerWsMessage::Reconnecting { error, .. }) =
        hydromancer_control_message("reconnecting", &data)
    else {
        panic!("expected reconnecting control message");
    };

    assert_eq!(
        error,
        "Hydromancer authentication failed. Check the API key in Settings > Integrations."
    );
}

#[test]
fn connecting_control_distinguishes_session_resume() {
    let data = serde_json::json!({
        "resuming": true,
    });

    assert!(matches!(
        hydromancer_control_message("connecting", &data),
        Some(HydromancerWsMessage::Resuming)
    ));
}
