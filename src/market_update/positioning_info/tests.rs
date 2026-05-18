use super::*;

#[test]
fn request_key_scopes_positioning_fetch_parameters() {
    assert_eq!(
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc"),
        "HYPE:all:unrealizedPnl:desc:30:0"
    );
    assert_ne!(
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc"),
        positioning_info_request_key("HYPE", "long", "unrealizedPnl", "desc")
    );
    assert_ne!(
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc"),
        positioning_info_request_key("HYPE", "all", "notionalSize", "desc")
    );
    assert_ne!(
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "desc"),
        positioning_info_request_key("HYPE", "all", "unrealizedPnl", "asc")
    );
}

#[test]
fn request_key_scopes_positioning_change_fetch_parameters() {
    assert_eq!(
        positioning_info_change_request_key("HYPE", "FIFTEEN_MINUTES"),
        "change:HYPE:FIFTEEN_MINUTES"
    );
    assert_ne!(
        positioning_info_change_request_key("HYPE", "FIFTEEN_MINUTES"),
        positioning_info_change_request_key("HYPE", "ONE_HOUR")
    );
    assert_ne!(
        positioning_info_change_request_key("HYPE", "FIFTEEN_MINUTES"),
        positioning_info_change_request_key("BTC", "FIFTEEN_MINUTES")
    );
}
