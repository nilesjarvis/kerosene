use super::*;

#[test]
fn market_stream_fallback_uses_auth_and_capacity_failures() {
    assert!(!hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Connecting
    ));
    assert!(!hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Reconnected
    ));
    assert!(!hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Reconnecting {
            error: "network timeout".to_string(),
            retry_delay_secs: 2,
        }
    ));
    assert!(hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Reconnecting {
            error:
                "Hydromancer authentication failed. Check the API key in Settings > Integrations."
                    .to_string(),
            retry_delay_secs: 2,
        }
    ));
    assert!(hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Reconnecting {
            error: "subscription rejected: too many subscriptions".to_string(),
            retry_delay_secs: 2,
        }
    ));
    assert!(hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Reconnecting {
            error: "subscription denied: quota exceeded".to_string(),
            retry_delay_secs: 2,
        }
    ));
    assert!(!hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Disconnected("stream disconnected".to_string())
    ));
    assert!(!hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Disconnected(
            "Hydromancer network timeout: heartbeat timeout after 95s".to_string()
        )
    ));
    assert!(hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Disconnected(
            "Hydromancer authentication failed. Check the API key in Settings > Integrations."
                .to_string()
        )
    ));
    assert!(hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Disconnected(
            "subscription rejected: too many subscriptions".to_string()
        )
    ));
    assert!(hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Disconnected(
            "subscription unsupported: quota exhausted".to_string()
        )
    ));
}

#[test]
fn capacity_notice_switches_market_adapters_but_leaves_required_feeds_running() {
    let data = serde_json::json!({});
    let control = hydromancer_market_control_message(MARKET_FALLBACK_MESSAGE, &data)
        .expect("market fallback control");
    assert!(hydromancer_market_control_should_fallback(&control));
    assert!(hydromancer_candle_control_message("1m", MARKET_FALLBACK_MESSAGE, &data).is_some());
    assert!(hydromancer_candle_control_message("1s", MARKET_FALLBACK_MESSAGE, &data).is_none());
    assert!(
        super::super::parsing::hydromancer_control_message(MARKET_FALLBACK_MESSAGE, &data)
            .is_none()
    );
}
