use super::*;

fn routed(msg_type: &str, json: Value) -> HydromancerRoutedMessage {
    HydromancerRoutedMessage {
        msg_type: msg_type.to_string(),
        data: Arc::new(json),
    }
}

fn receive(
    receiver: &mut broadcast::Receiver<HydromancerRoutedMessage>,
) -> HydromancerRoutedMessage {
    receiver.try_recv().expect("message should be routed")
}

#[test]
fn non_book_messages_pass_through_immediately() {
    let (sender, mut receiver) = broadcast::channel(8);
    let mut coalescer = HydromancerCoalescedSender::with_interval(sender, Duration::from_secs(60));

    coalescer.submit(routed(
        "activeAssetCtx",
        serde_json::json!({"type": "activeAssetCtx", "data": {"coin": "BTC"}}),
    ));

    let message = receive(&mut receiver);
    assert_eq!(message.msg_type, "activeAssetCtx");
    assert_eq!(message.data["type"], "activeAssetCtx");
}

#[test]
fn book_messages_keep_latest_snapshot_per_coin() {
    let (sender, mut receiver) = broadcast::channel(8);
    let mut coalescer = HydromancerCoalescedSender::with_interval(sender, Duration::from_secs(60));

    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({"type": "l2Book", "data": {"coin": "BTC", "seq": 1}}),
    ));
    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({"type": "l2Book", "data": {"coin": "BTC", "seq": 2}}),
    ));
    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({"type": "l2Book", "data": {"coin": "BTC", "seq": 3}}),
    ));

    assert_eq!(receive(&mut receiver).data["data"]["seq"], 1);
    assert!(receiver.try_recv().is_err());

    assert_eq!(coalescer.flush_all(), 1);
    assert_eq!(receive(&mut receiver).data["data"]["seq"], 3);
}

#[test]
fn book_messages_are_coalesced_independently_by_coin() {
    let (sender, mut receiver) = broadcast::channel(8);
    let mut coalescer = HydromancerCoalescedSender::with_interval(sender, Duration::from_secs(60));

    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({"type": "l2Book", "data": {"coin": "BTC", "seq": 1}}),
    ));
    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({"type": "l2Book", "data": {"coin": "ETH", "seq": 1}}),
    ));
    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({"type": "l2Book", "data": {"coin": "BTC", "seq": 2}}),
    ));

    assert_eq!(receive(&mut receiver).data["data"]["coin"], "BTC");
    assert_eq!(receive(&mut receiver).data["data"]["coin"], "ETH");
    assert!(receiver.try_recv().is_err());

    assert_eq!(coalescer.flush_all(), 1);
    let pending = receive(&mut receiver);
    assert_eq!(pending.data["data"]["coin"], "BTC");
    assert_eq!(pending.data["data"]["seq"], 2);
}

#[test]
fn multi_coin_book_batches_pass_through() {
    let (sender, mut receiver) = broadcast::channel(8);
    let mut coalescer = HydromancerCoalescedSender::with_interval(sender, Duration::from_secs(60));

    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({
            "type": "l2Book",
            "data": [
                {"coin": "BTC", "seq": 1},
                {"coin": "ETH", "seq": 1}
            ]
        }),
    ));
    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({
            "type": "l2Book",
            "data": [
                {"coin": "BTC", "seq": 2},
                {"coin": "ETH", "seq": 2}
            ]
        }),
    ));

    assert_eq!(receive(&mut receiver).data["data"][0]["seq"], 1);
    assert_eq!(receive(&mut receiver).data["data"][0]["seq"], 2);
    assert_eq!(coalescer.flush_all(), 0);
}
