use super::*;

#[test]
fn all_mids_parser_drops_invalid_prices() {
    let payload = serde_json::json!({
        "mids": {
            "BTC": "100.5",
            "BAD": "not-a-price",
            "NAN": "NaN",
            "INF": "inf",
            "ZERO": "0",
            "NEG": "-1"
        }
    });

    let Some((source_addr, WsUserData::AllMids(mids))) =
        parse_user_stream_message("allMids", &payload, None, Some("0xabc".to_string()))
    else {
        panic!("expected all mids update");
    };

    assert_eq!(source_addr.as_deref(), Some("0xabc"));
    assert_eq!(mids.get("BTC"), Some(&100.5));
    assert!(!mids.contains_key("BAD"));
    assert!(!mids.contains_key("NAN"));
    assert!(!mids.contains_key("INF"));
    assert!(!mids.contains_key("ZERO"));
    assert!(!mids.contains_key("NEG"));
}
