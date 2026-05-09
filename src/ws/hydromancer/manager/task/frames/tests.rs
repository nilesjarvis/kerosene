use super::{HydromancerTextFrameKind, parse_hydromancer_text_frame};

#[test]
fn text_frame_parser_rejects_invalid_json_or_missing_type() {
    assert!(parse_hydromancer_text_frame("not json").is_none());
    assert!(parse_hydromancer_text_frame(r#"{"cursor":"abc"}"#).is_none());
}

#[test]
fn text_frame_parser_classifies_connected_with_session_and_cursor() {
    let frame = parse_hydromancer_text_frame(
        r#"{"type":"connected","sessionId":"session-1","cursor":"cursor-1"}"#,
    )
    .expect("connected frame");

    assert_eq!(frame.kind, HydromancerTextFrameKind::Connected);
    assert_eq!(frame.session_id.as_deref(), Some("session-1"));
    assert_eq!(frame.cursor.as_deref(), Some("cursor-1"));
}

#[test]
fn text_frame_parser_classifies_reconnected_and_ping() {
    let reconnected =
        parse_hydromancer_text_frame(r#"{"type":"reconnected"}"#).expect("reconnected frame");
    let ping = parse_hydromancer_text_frame(r#"{"type":"ping"}"#).expect("ping frame");

    assert_eq!(reconnected.kind, HydromancerTextFrameKind::Reconnected);
    assert_eq!(ping.kind, HydromancerTextFrameKind::Ping);
}

#[test]
fn text_frame_parser_preserves_unknown_typed_frames_as_other() {
    let frame =
        parse_hydromancer_text_frame(r#"{"type":"userFills","data":[]}"#).expect("data frame");

    assert_eq!(frame.kind, HydromancerTextFrameKind::Other);
    assert_eq!(frame.json["type"], "userFills");
}
