use super::*;

const INTERVAL: Duration = Duration::from_secs(3600);

#[test]
fn pending_updates_keep_their_deadline_and_flush_only_due_keys() {
    let (tx, mut rx) = broadcast::channel(16);
    let mut sender = SnapshotCoalescer::new(tx, INTERVAL);
    sender.submit(Some("BTC"), 1);
    sender.submit(Some("ETH"), 2);
    assert_eq!(rx.try_recv().expect("first BTC"), 1);
    assert_eq!(rx.try_recv().expect("first ETH"), 2);
    sender.submit(Some("BTC"), 3);
    sender.submit(Some("ETH"), 4);
    let deadline = sender.pending["BTC"].deadline;
    sender.submit(Some("BTC"), 5);
    assert_eq!(sender.pending["BTC"].deadline, deadline);
    assert!(rx.try_recv().is_err());
    assert_eq!(sender.flush_due(), 0);

    sender.pending.get_mut("BTC").expect("pending BTC").deadline = Instant::now();
    assert_eq!(sender.next_due(), Some(Duration::ZERO));
    assert_eq!(sender.flush_due(), 1);
    assert_eq!(rx.try_recv().expect("latest BTC"), 5);
    assert!(rx.try_recv().is_err());
    assert_eq!(sender.pending.len(), 1);

    // A flush starts a new pacing interval for the emitted key.
    sender.submit(Some("BTC"), 6);
    assert!(rx.try_recv().is_err());
    assert_eq!(sender.flush_all(), 2);
    let mut flushed = vec![
        rx.try_recv().expect("first book"),
        rx.try_recv().expect("second book"),
    ];
    flushed.sort_unstable();
    assert_eq!(flushed, vec![4, 6]);
    assert_eq!(sender.next_due(), None);
    assert_eq!(sender.flush_all(), 0);
}

#[test]
fn pruning_keeps_pending_keys_and_discards_only_expired_idle_history() {
    let (tx, mut rx) = broadcast::channel(16);
    let mut sender = SnapshotCoalescer::new(tx, INTERVAL);
    sender.submit(Some("pending"), 1);
    sender.submit(Some("idle"), 2);
    sender.submit(Some("pending"), 3);
    assert_eq!(rx.try_recv().expect("first pending key"), 1);
    assert_eq!(rx.try_recv().expect("idle key"), 2);
    let expired = Instant::now() - INTERVAL * 2;
    sender.last_emitted.insert("pending", expired);
    sender.last_emitted.insert("idle", expired);
    sender
        .pending
        .get_mut("pending")
        .expect("pending snapshot")
        .deadline = expired + INTERVAL;

    sender.submit(Some("new"), 4);
    assert_eq!(rx.try_recv().expect("new key"), 4);
    assert!(sender.last_emitted.contains_key("pending"));
    assert!(!sender.last_emitted.contains_key("idle"));
    assert_eq!(sender.flush_due(), 1);
    assert_eq!(rx.try_recv().expect("retained pending snapshot"), 3);
}

#[test]
fn new_immediate_snapshot_supersedes_pending_and_zero_interval_never_queues() {
    let (tx, mut rx) = broadcast::channel(16);
    let mut sender = SnapshotCoalescer::new(tx, INTERVAL);
    sender.submit(Some("BTC"), 1);
    sender.submit(Some("BTC"), 2);
    assert_eq!(rx.try_recv().expect("first snapshot"), 1);
    sender
        .last_emitted
        .insert("BTC", Instant::now() - INTERVAL * 2);
    sender.submit(Some("BTC"), 3);
    assert_eq!(rx.try_recv().expect("newer immediate snapshot"), 3);
    assert_eq!(sender.flush_all(), 0);

    let (tx, mut rx) = broadcast::channel(16);
    let mut sender = SnapshotCoalescer::new(tx, Duration::ZERO);
    for value in 0..3 {
        sender.submit(Some("BTC"), value);
        assert_eq!(rx.try_recv().expect("unpaced snapshot"), value);
    }
    assert_eq!(sender.next_due(), None);
}

#[test]
fn unkeyed_messages_bypass_pacing_and_failed_broadcasts_still_record_emission() {
    let (tx, rx) = broadcast::channel(16);
    drop(rx);
    let mut sender = SnapshotCoalescer::new(tx.clone(), INTERVAL);
    sender.submit(Some("BTC"), 1);
    let mut rx = tx.subscribe();
    sender.submit(Some("BTC"), 2);
    assert!(
        rx.try_recv().is_err(),
        "failed send still starts the pacing interval"
    );
    sender.submit(None, 3);
    assert_eq!(rx.try_recv().expect("unpaced message"), 3);
    assert_eq!(sender.history_len(), 1);
    assert_eq!(sender.flush_all(), 1);
    assert_eq!(rx.try_recv().expect("latest snapshot"), 2);
}
