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

fn receive_sequences_by_coin(
    receiver: &mut broadcast::Receiver<HydromancerRoutedMessage>,
    count: usize,
) -> std::collections::HashMap<String, i64> {
    let mut sequences = std::collections::HashMap::new();
    for _ in 0..count {
        let message = receive(receiver);
        let coin = message
            .data
            .get("coin")
            .and_then(Value::as_str)
            .expect("split book item should include coin")
            .to_string();
        let seq = message
            .data
            .get("seq")
            .and_then(Value::as_i64)
            .expect("split book item should include seq");
        sequences.insert(coin, seq);
    }
    sequences
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
fn immediate_book_emit_drops_older_pending_for_same_key() {
    let (sender, mut receiver) = broadcast::channel(8);
    let mut coalescer =
        HydromancerCoalescedSender::with_interval(sender, Duration::from_millis(50));

    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({"type": "l2Book", "data": {"coin": "BTC", "seq": 1}}),
    ));
    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({"type": "l2Book", "data": {"coin": "BTC", "seq": 2}}),
    ));

    assert_eq!(receive(&mut receiver).data["data"]["seq"], 1);
    assert!(receiver.try_recv().is_err());

    std::thread::sleep(Duration::from_millis(70));
    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({"type": "l2Book", "data": {"coin": "BTC", "seq": 3}}),
    ));

    assert_eq!(
        receive(&mut receiver).data["data"]["seq"],
        3,
        "newer immediate emit should reach subscribers before any flush"
    );
    assert_eq!(
        coalescer.flush_due(),
        0,
        "older pending book should be removed when a newer book emits"
    );
    assert!(receiver.try_recv().is_err());
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
fn multi_coin_book_batches_are_coalesced_independently_by_coin() {
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

    let first_batch = receive_sequences_by_coin(&mut receiver, 2);
    assert_eq!(first_batch.get("BTC"), Some(&1));
    assert_eq!(first_batch.get("ETH"), Some(&1));
    assert!(receiver.try_recv().is_err());

    assert_eq!(coalescer.flush_all(), 2);
    let second_batch = receive_sequences_by_coin(&mut receiver, 2);
    assert_eq!(second_batch.get("BTC"), Some(&2));
    assert_eq!(second_batch.get("ETH"), Some(&2));
}

#[test]
fn ambiguous_book_batches_pass_through_without_splitting() {
    let (sender, mut receiver) = broadcast::channel(8);
    let mut coalescer = HydromancerCoalescedSender::with_interval(sender, Duration::from_secs(60));

    coalescer.submit(routed(
        "l2Book",
        serde_json::json!({
            "type": "l2Book",
            "data": [
                {"coin": "BTC", "seq": 1},
                {"seq": 2}
            ]
        }),
    ));

    let message = receive(&mut receiver);
    assert_eq!(message.data["data"][0]["seq"], 1);
    assert_eq!(message.data["data"][1]["seq"], 2);
    assert_eq!(coalescer.flush_all(), 0);
}
