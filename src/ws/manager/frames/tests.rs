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
