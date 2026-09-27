use super::*;

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
fn books_without_coin_keep_their_shared_slot_and_distinct_precision() {
    let (tx, mut rx) = broadcast::channel(16);
    let mut sender = CoalescedSender::with_interval(tx, Duration::from_secs(60));
    for data in [
        json!({"seq": 1}),
        json!({"seq": 2}),
        json!({"seq": 3, "nSigFigs": 5}),
    ] {
        sender.submit("l2Book".to_string(), Arc::new(data));
    }
    let immediate = drain(&mut rx);
    assert_eq!(immediate.len(), 2);
    assert_eq!(immediate[0].1["seq"], 1);
    assert_eq!(immediate[1].1["seq"], 3);
    assert_eq!(sender.flush_all(), 1);
    let flushed = drain(&mut rx);
    assert_eq!(flushed.len(), 1);
    assert_eq!(flushed[0].0, "l2Book");
    assert_eq!(flushed[0].1["seq"], 2);
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
