use super::*;

#[test]
fn book_updates_within_interval_collapse_to_latest_wins() {
    let (tx, mut rx) = broadcast::channel(16);
    let mut sender = CoalescedSender::with_interval(tx, Duration::from_millis(200));

    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "BTC", "seq": 1 })),
    );
    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "BTC", "seq": 2 })),
    );
    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "BTC", "seq": 3 })),
    );

    let drained_before = drain(&mut rx);
    assert_eq!(
        drained_before.len(),
        1,
        "only the very first should be live"
    );
    assert_eq!(
        drained_before[0].1.get("seq").and_then(|v| v.as_i64()),
        Some(1)
    );
    assert!(sender.next_due().is_some());

    std::thread::sleep(Duration::from_millis(220));
    let flushed = sender.flush_due();
    assert_eq!(flushed, 1);
    let drained_after = drain(&mut rx);
    assert_eq!(drained_after.len(), 1);
    assert_eq!(
        drained_after[0].1.get("seq").and_then(|v| v.as_i64()),
        Some(3),
        "latest pending payload wins"
    );
}
