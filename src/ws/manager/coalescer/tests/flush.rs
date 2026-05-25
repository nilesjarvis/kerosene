use super::*;

#[test]
fn flush_due_clears_only_expired_entries() {
    let (tx, mut rx) = broadcast::channel(16);
    let mut sender = CoalescedSender::with_interval(tx, Duration::from_millis(100));

    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "BTC", "seq": 1 })),
    );
    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "BTC", "seq": 2 })),
    );
    let _ = drain(&mut rx);

    let flushed_early = sender.flush_due();
    assert_eq!(flushed_early, 0, "nothing has expired yet");

    std::thread::sleep(Duration::from_millis(120));
    let flushed_after = sender.flush_due();
    assert_eq!(flushed_after, 1);
    assert!(sender.next_due().is_none());
}

#[test]
fn flush_all_emits_pending_entries_before_disconnect() {
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
    let _ = drain(&mut rx);

    let flushed = sender.flush_all();
    assert_eq!(flushed, 1);
    assert!(sender.next_due().is_none());

    let drained = drain(&mut rx);
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].1.get("seq").and_then(|v| v.as_i64()), Some(2));
}
