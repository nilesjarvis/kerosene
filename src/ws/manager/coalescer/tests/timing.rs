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
