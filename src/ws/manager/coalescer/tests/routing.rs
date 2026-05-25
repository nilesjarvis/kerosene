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
