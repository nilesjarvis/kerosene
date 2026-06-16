use serde_json::json;

use super::build_user_stream_subscriptions;

#[test]
fn subscriptions_without_address_include_global_and_dex_mids_only() {
    let subscriptions =
        build_user_stream_subscriptions(None, &["".to_string(), "dex-a".to_string()], true);

    assert_eq!(
        subscriptions
            .iter()
            .map(|(topic, _)| topic.as_str())
            .collect::<Vec<_>>(),
        vec!["allMids", "allMids:dex-a"]
    );
    assert_eq!(
        subscriptions[1].1,
        json!({
            "method": "subscribe",
            "subscription": { "type": "allMids", "dex": "dex-a" }
        })
    );
}

#[test]
fn subscriptions_with_address_include_private_streams_and_dex_orders() {
    let subscriptions =
        build_user_stream_subscriptions(Some("0xabc"), &["dex-a".to_string()], true);

    assert_eq!(
        subscriptions
            .iter()
            .map(|(topic, _)| topic.as_str())
            .collect::<Vec<_>>(),
        vec![
            "allMids",
            "allMids:dex-a",
            "allDexsClearinghouseState:0xabc",
            "userFills:0xabc",
            "spotState:0xabc",
            "openOrders:0xabc:dex-a",
        ]
    );
    assert_eq!(
        subscriptions[5].1,
        json!({
            "method": "subscribe",
            "subscription": { "type": "openOrders", "user": "0xabc", "dex": "dex-a" }
        })
    );
}

#[test]
fn subscriptions_with_main_dex_include_main_open_orders() {
    let subscriptions = build_user_stream_subscriptions(
        Some("0xabc"),
        &["".to_string(), "dex-a".to_string()],
        true,
    );

    assert!(
        subscriptions
            .iter()
            .any(|(topic, _)| topic == "openOrders:0xabc")
    );
}

#[test]
fn subscriptions_without_mids_include_private_streams_only() {
    let subscriptions = build_user_stream_subscriptions(
        Some("0xabc"),
        &["".to_string(), "dex-a".to_string()],
        false,
    );

    assert_eq!(
        subscriptions
            .iter()
            .map(|(topic, _)| topic.as_str())
            .collect::<Vec<_>>(),
        vec![
            "allDexsClearinghouseState:0xabc",
            "userFills:0xabc",
            "spotState:0xabc",
            "openOrders:0xabc",
            "openOrders:0xabc:dex-a",
        ]
    );
}
