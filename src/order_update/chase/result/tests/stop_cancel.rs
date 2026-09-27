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
fn stopped_chase_cancel_request_debug_redacts_agent_key() {
    let request = StoppedChaseCancelRequest {
        chase_id: 1,
        agent_key: "agent-secret".to_string().into(),
        asset: 7,
        oid: 9001,
    };

    let rendered = format!("{request:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains("agent-secret"));
}

#[test]
fn stopped_subaccount_chase_cancel_retains_original_target_and_key() {
    let mut chase = chase();
    chase.agent_key = crate::signing::CapturedAgentKey::for_account(
        "parent-agent-secret".to_string().into(),
        Some(TEST_ACCOUNT),
    )
    .expect("subaccount key");
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::AwaitingPlace,
    };
    let response = exchange_response(vec![serde_json::json!({"resting": {"oid": 9001_u64}})]);

    let request = stopped_chase_cancel_request(&chase, &response)
        .expect("late resting subaccount order must be cancelled");
    chase.agent_key.clear();

    assert_eq!(request.agent_key.as_str(), "parent-agent-secret");
    assert_eq!(request.agent_key.vault_address(), Some(TEST_ACCOUNT));
    assert_eq!(request.asset, 7);
    assert_eq!(request.oid, 9001);
    assert!(!format!("{request:?}").contains(TEST_ACCOUNT));
    assert!(!format!("{request:?}").contains("parent-agent-secret"));
    assert!(stopped_chase_cancel_request(&chase, &response).is_none());
}
