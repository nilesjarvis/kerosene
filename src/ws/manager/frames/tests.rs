use serde_json::json;

use super::{WsTextFrame, parse_ws_text_frame};

#[test]
fn text_frame_parser_detects_pong() {
    assert_eq!(
        parse_ws_text_frame(r#"{"channel":"pong"}"#),
        WsTextFrame::Pong
    );
}

#[test]
fn text_frame_parser_extracts_channel_data() {
    assert_eq!(
        parse_ws_text_frame(r#"{"channel":"trades","data":[{"px":"1"}]}"#),
        WsTextFrame::Data {
            channel: "trades".to_string(),
            data: json!([{"px":"1"}]),
        }
    );
}

#[test]
fn text_frame_parser_ignores_invalid_or_incomplete_frames() {
    assert_eq!(parse_ws_text_frame("not-json"), WsTextFrame::Ignored);
    assert_eq!(parse_ws_text_frame(r#"{"data":[]}"#), WsTextFrame::Ignored);
    assert_eq!(
        parse_ws_text_frame(r#"{"channel":"trades"}"#),
        WsTextFrame::Ignored
    );
}

#[test]
fn text_frame_parser_preserves_all_json_data_shapes_and_pong_precedence() {
    for data in [
        json!(null),
        json!(false),
        json!(42),
        json!("snapshot"),
        json!([]),
        json!({"coin": "BTC", "levels": [[{"px": "100", "sz": "2"}], []]}),
    ] {
        let frame = json!({"channel": "l2Book", "data": data, "extra": "ignored"});
        assert_eq!(
            parse_ws_text_frame(&frame.to_string()),
            WsTextFrame::Data {
                channel: "l2Book".to_string(),
                data: data.clone()
            }
        );
        assert_eq!(
            parse_ws_text_frame(&json!({"channel": "pong", "data": data}).to_string()),
            WsTextFrame::Pong
        );
    }
    for frame in [
        json!(null),
        json!([{"channel": "l2Book", "data": []}]),
        json!({"channel": null, "data": []}),
        json!({"channel": 42, "data": []}),
    ] {
        assert_eq!(
            parse_ws_text_frame(&frame.to_string()),
            WsTextFrame::Ignored
        );
    }
}

#[test]
fn text_frame_debug_redacts_raw_data() {
    let address = "0xabc0000000000000000000000000000000000000";
    let frame = parse_ws_text_frame(&format!(
        r#"{{"channel":"userFills","data":{{"user":"{address}","hash":"fill-secret"}}}}"#
    ));

    let rendered = format!("{frame:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains(address));
    assert!(!rendered.contains("fill-secret"));
}
