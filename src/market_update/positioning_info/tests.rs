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
