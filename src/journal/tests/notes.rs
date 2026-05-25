use super::fill;
use crate::journal::note_key_for_trade;
use crate::journal::{JournalNote, aggregate_trades_with_diagnostics, note_for_trade};
use std::collections::HashMap;

#[test]
fn note_lookup_keeps_legacy_time_based_keys_working() {
    let result = aggregate_trades_with_diagnostics(vec![fill(1, 10, "BTC")]);
    let trade = &result.trades[0];
    let legacy_key = "BTC_1".to_string();
    let mut entries = HashMap::new();
    entries.insert(
        legacy_key.clone(),
        JournalNote {
            open: "legacy note".to_string(),
            close: String::new(),
        },
    );

    assert_ne!(trade.id, legacy_key);
    assert_eq!(note_key_for_trade(&entries, trade), Some(legacy_key));
    assert_eq!(
        note_for_trade(&entries, trade).map(|note| note.open.as_str()),
        Some("legacy note")
    );
}
