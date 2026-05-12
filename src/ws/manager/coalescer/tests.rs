use super::*;
use serde_json::json;
use tokio::sync::broadcast;

fn drain(rx: &mut broadcast::Receiver<WsRoutedMessage>) -> Vec<(String, Arc<Value>)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push((msg.channel, msg.data));
    }
    out
}

#[test]
fn non_coalesced_channels_pass_through_immediately() {
    let (tx, mut rx) = broadcast::channel(16);
    let mut sender = CoalescedSender::new(tx);

    for n in 0..3 {
        sender.submit("userFills".to_string(), Arc::new(json!({ "n": n })));
    }

    let drained = drain(&mut rx);
    assert_eq!(drained.len(), 3);
    assert!(sender.next_due().is_none(), "pass-through never queues");
}

#[test]
fn first_book_update_per_coin_emits_immediately() {
    let (tx, mut rx) = broadcast::channel(16);
    let mut sender = CoalescedSender::new(tx);

    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "BTC", "levels": [] })),
    );

    let drained = drain(&mut rx);
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].0, "l2Book");
}

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

    // Wait past the deadline + flush.
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

#[test]
fn different_coins_do_not_collapse_into_one_slot() {
    let (tx, mut rx) = broadcast::channel(16);
    let mut sender = CoalescedSender::with_interval(tx, Duration::from_millis(200));

    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "BTC", "seq": 1 })),
    );
    sender.submit(
        "l2Book".to_string(),
        Arc::new(json!({ "coin": "ETH", "seq": 1 })),
    );

    let drained = drain(&mut rx);
    assert_eq!(drained.len(), 2, "first frame per coin emits immediately");
    let coins: Vec<&str> = drained
        .iter()
        .filter_map(|(_, v)| v.get("coin").and_then(|c| c.as_str()))
        .collect();
    assert!(coins.contains(&"BTC"));
    assert!(coins.contains(&"ETH"));
}

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

    let initial = sender.next_due().expect("pending entry should exist");
    assert!(initial <= Duration::from_millis(50));

    std::thread::sleep(Duration::from_millis(70));
    let after = sender
        .next_due()
        .expect("entry is still pending until flushed");
    assert_eq!(after, Duration::ZERO);
}

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
