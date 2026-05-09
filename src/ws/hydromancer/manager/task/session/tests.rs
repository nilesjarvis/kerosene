use crate::ws::hydromancer::manager::task::frames::parse_hydromancer_text_frame;

use super::*;

#[test]
fn connecting_data_reports_resume_and_cursor_state() {
    let mut session = HydromancerSessionState::default();
    assert_eq!(
        session.connecting_data(),
        serde_json::json!({"resuming": false, "hasCursor": false})
    );

    let frame = parse_hydromancer_text_frame(
        r#"{"type":"connected","sessionId":"session-1","cursor":"cursor-1"}"#,
    )
    .expect("connected frame");
    session.apply_text_frame(&frame);

    assert_eq!(
        session.connecting_data(),
        serde_json::json!({"resuming": true, "hasCursor": true})
    );
}

#[test]
fn connected_frame_marks_ready_and_replays_subscriptions() {
    let mut session = HydromancerSessionState::default();
    let frame = parse_hydromancer_text_frame(
        r#"{"type":"connected","sessionId":"session-1","cursor":"cursor-1"}"#,
    )
    .expect("connected frame");

    let action = session.apply_text_frame(&frame);

    assert!(session.connection_ready());
    assert_eq!(session.session_id(), Some("session-1"));
    assert_eq!(session.last_cursor(), Some("cursor-1"));
    assert_eq!(
        action,
        HydromancerFrameAction {
            resend_subscriptions: true,
            send_pong: false
        }
    );
}

#[test]
fn reconnected_frame_marks_ready_without_subscription_replay() {
    let mut session = HydromancerSessionState::default();
    let frame = parse_hydromancer_text_frame(r#"{"type":"reconnected","sessionId":"next"}"#)
        .expect("reconnected frame");

    let action = session.apply_text_frame(&frame);

    assert!(session.connection_ready());
    assert_eq!(session.session_id(), Some("next"));
    assert_eq!(
        action,
        HydromancerFrameAction {
            resend_subscriptions: false,
            send_pong: false
        }
    );
}

#[test]
fn ping_frame_requests_pong_and_can_update_cursor() {
    let mut session = HydromancerSessionState::default();
    let frame =
        parse_hydromancer_text_frame(r#"{"type":"ping","cursor":"cursor-2"}"#).expect("ping");

    let action = session.apply_text_frame(&frame);

    assert!(!session.connection_ready());
    assert_eq!(session.last_cursor(), Some("cursor-2"));
    assert_eq!(
        action,
        HydromancerFrameAction {
            resend_subscriptions: false,
            send_pong: true
        }
    );
}

#[test]
fn begin_connection_clears_ready_without_forgetting_resume_state() {
    let mut session = HydromancerSessionState::default();
    let frame = parse_hydromancer_text_frame(
        r#"{"type":"connected","sessionId":"session-1","cursor":"cursor-1"}"#,
    )
    .expect("connected frame");
    session.apply_text_frame(&frame);

    session.begin_connection();

    assert!(!session.connection_ready());
    assert_eq!(session.session_id(), Some("session-1"));
    assert_eq!(session.last_cursor(), Some("cursor-1"));
}
