use super::*;

#[test]
fn next_due_returns_zero_once_deadline_has_passed() {
    let (tx, _rx) = broadcast::channel(16);
    let mut sender = CoalescedSender::with_interval(tx, Duration::from_millis(50));

    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "BTC", "seq": 1 })),
    );
    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "BTC", "seq": 2 })),
    );

    let initial = next_due(&sender, "pending entry should exist");
    assert!(initial <= Duration::from_millis(50));

    std::thread::sleep(Duration::from_millis(70));
    let after = next_due(&sender, "entry is still pending until flushed");
    assert_eq!(after, Duration::ZERO);
}

#[test]
fn stale_last_emitted_history_is_pruned_on_book_submit() {
    let (tx, mut rx) = broadcast::channel(16);
    let mut sender = CoalescedSender::with_interval(tx, Duration::from_millis(10));

    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "BTC", "seq": 1 })),
    );
    assert_eq!(sender.last_emitted.len(), 1);

    std::thread::sleep(Duration::from_millis(25));
    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "ETH", "seq": 1 })),
    );

    assert_eq!(sender.last_emitted.len(), 1);
    let drained = drain(&mut rx);
    assert_eq!(drained.len(), 2);
    assert_eq!(
        drained[1].1.get("coin").and_then(|coin| coin.as_str()),
        Some("ETH")
    );
    assert!(sender.next_due().is_none());
}
