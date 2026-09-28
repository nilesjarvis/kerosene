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
    assert_eq!(
        frame.session_id.as_ref().map(|value| value.as_str()),
        Some("session-1")
    );
    assert_eq!(
        frame.cursor.as_ref().map(|value| value.as_str()),
        Some("cursor-1")
    );
    assert!(frame.json.get("sessionId").is_none());
    assert!(frame.json.get("cursor").is_none());
}

#[test]
fn text_frame_debug_redacts_resume_material() {
    let frame = parse_hydromancer_text_frame(
        r#"{"type":"connected","sessionId":"session-secret","cursor":"cursor-secret"}"#,
    )
    .expect("connected frame");

    let rendered = format!("{frame:?}");

    assert!(rendered.contains("has_cursor: true"));
    assert!(rendered.contains("has_session_id: true"));
    assert!(!rendered.contains("session-secret"));
    assert!(!rendered.contains("cursor-secret"));
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

#[test]
fn text_frame_parser_removes_non_string_resume_fields_without_touching_data() {
    let payload = serde_json::json!({
        "type": "userFills",
        "data": {"cursor": "event-cursor", "sessionId": "event-session", "fills": []},
    });
    for value in [
        serde_json::Value::Null,
        serde_json::json!(true),
        serde_json::json!(42),
        serde_json::json!(["resume-field"]),
        serde_json::json!({"value": "resume-field"}),
    ] {
        let mut input = payload.clone();
        input["cursor"] = value.clone();
        input["sessionId"] = value;

        let frame = parse_hydromancer_text_frame(&input.to_string()).expect("typed frame");

        assert_eq!(frame.kind, HydromancerTextFrameKind::Other);
        assert!(frame.cursor.is_none());
        assert!(frame.session_id.is_none());
        assert_eq!(frame.json, payload);
    }
}

#[test]
fn text_frame_parser_preserves_empty_whitespace_and_unicode_resume_strings() {
    for value in ["", "  resume field\n", "resume-雪-⚡"] {
        let input = serde_json::json!({
            "type": "ping",
            "cursor": value,
            "sessionId": value,
            "timestamp": 42,
        });

        let frame = parse_hydromancer_text_frame(&input.to_string()).expect("ping frame");

        assert_eq!(frame.kind, HydromancerTextFrameKind::Ping);
        assert_eq!(frame.cursor.as_deref().map(String::as_str), Some(value));
        assert_eq!(frame.session_id.as_deref().map(String::as_str), Some(value));
        assert_eq!(
            frame.json,
            serde_json::json!({"type": "ping", "timestamp": 42})
        );
    }
}
