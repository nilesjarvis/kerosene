use super::*;

#[test]
fn stopped_chase_place_result_requests_cancel_for_late_resting_order() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::AwaitingPlace,
    };
    let response = exchange_response(vec![serde_json::json!({
        "resting": {
            "oid": 9001_u64
        }
    })]);

    assert_eq!(
        stopped_chase_cancel_request(&chase, &response),
        Some(StoppedChaseCancelRequest {
            chase_id: 1,
            agent_key: "agent-key".to_string().into(),
            asset: 7,
            oid: 9001
        })
    );
}

#[test]
fn active_chase_place_result_does_not_request_stop_cancel() {
    let chase = chase();
    let response = exchange_response(vec![serde_json::json!({
        "resting": {
            "oid": 9001_u64
        }
    })]);

    assert_eq!(stopped_chase_cancel_request(&chase, &response), None);
}

#[test]
fn stopped_chase_cancel_request_debug_redacts_agent_key_and_oid() {
    let request = StoppedChaseCancelRequest {
        chase_id: 1,
        agent_key: "agent-secret".to_string().into(),
        asset: 7,
        oid: 9001,
    };

    let rendered = format!("{request:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains("agent-secret"));
    assert!(!rendered.contains("9001"));
}
