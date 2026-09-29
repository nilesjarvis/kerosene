use super::*;

#[test]
fn market_stream_fallback_only_uses_auth_failures() {
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
    assert!(!hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Reconnecting {
            error: "subscription rejected: too many subscriptions".to_string(),
            retry_delay_secs: 2,
        }
    ));
    assert!(!hydromancer_market_control_should_fallback(
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
    assert!(!hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Disconnected(
            "subscription rejected: too many subscriptions".to_string()
        )
    ));
    assert!(!hydromancer_market_control_should_fallback(
        &HydromancerWsMessage::Disconnected(
            "subscription unsupported: quota exhausted".to_string()
        )
    ));
}
