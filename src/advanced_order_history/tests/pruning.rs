use super::*;
use std::collections::VecDeque;

#[test]
fn advanced_order_history_upsert_replaces_and_prunes_invalid_or_old_entries() {
    let mut history = VecDeque::new();
    let mut entry = minimal_entry("one");
    upsert_advanced_order_history(&mut history, entry.clone());
    assert_eq!(history.len(), 1);

    entry.summary = "updated".to_string();
    upsert_advanced_order_history(&mut history, entry);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].summary, "updated");

    history.push_back(minimal_entry(""));
    for index in 0..(ADVANCED_ORDER_HISTORY_LIMIT + 2) {
        history.push_back(minimal_entry(&format!("old-{index}")));
    }
    prune_advanced_order_history(&mut history);

    assert_eq!(history.len(), ADVANCED_ORDER_HISTORY_LIMIT);
    assert!(history.iter().all(|entry| !entry.id.trim().is_empty()));
}
