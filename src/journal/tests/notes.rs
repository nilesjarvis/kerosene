use super::{fill, note};
use crate::journal::{
    JournalNote, aggregate_trades_with_diagnostics, journal_tags_input, note_entry_for_trade,
    note_for_trade, parse_journal_tags,
};
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
            ..Default::default()
        },
    );

    assert_ne!(trade.id, legacy_key);
    assert_eq!(
        note_entry_for_trade(&entries, trade).map(|(key, _)| key),
        Some(legacy_key.as_str())
    );
    assert_eq!(
        note_for_trade(&entries, trade).map(|note| note.open.as_str()),
        Some("legacy note")
    );
}

#[test]
fn note_lookup_prefers_current_id_even_when_its_note_is_empty() {
    let result = aggregate_trades_with_diagnostics(vec![fill(1, 10, "BTC")]);
    let trade = &result.trades[0];
    let entries = HashMap::from([
        (trade.id.clone(), JournalNote::default()),
        ("BTC_1".to_string(), note("legacy note")),
    ]);

    assert_eq!(
        note_entry_for_trade(&entries, trade).map(|(key, _)| key),
        Some(trade.id.as_str())
    );
    assert!(note_for_trade(&entries, trade).is_some_and(JournalNote::is_empty));
}

#[test]
fn note_lookup_uses_first_exact_legacy_match_and_returns_none_when_missing() {
    let mut result = aggregate_trades_with_diagnostics(vec![fill(1, 10, "BTC")]);
    let trade = &mut result.trades[0];
    trade.legacy_note_ids = ["missing", "BTC_2", "BTC_1", "BTC_2"]
        .map(str::to_string)
        .to_vec();
    let mut entries = HashMap::from([
        ("BTC_1".to_string(), note("later alias")),
        ("BTC_2".to_string(), note("first alias")),
        ("btc_2".to_string(), note("different case")),
    ]);

    for (expected_key, expected_text) in [("BTC_2", "first alias"), ("BTC_1", "later alias")] {
        assert_eq!(
            note_entry_for_trade(&entries, trade).map(|(key, _)| key),
            Some(expected_key)
        );
        let selected = note_for_trade(&entries, trade).expect("matching legacy note");
        assert_eq!(selected.open, expected_text);
        assert!(std::ptr::eq(selected, &entries[expected_key]));
        entries.remove(expected_key);
    }

    assert!(note_entry_for_trade(&entries, trade).is_none());
    assert!(note_for_trade(&entries, trade).is_none());
}

#[test]
fn legacy_string_note_deserializes_without_tags() {
    let note: JournalNote = serde_json::from_str("\"just text\"").expect("legacy note");
    assert_eq!(note.open, "just text");
    assert!(note.close.is_empty());
    assert!(note.cause_of_error.is_empty());
    assert!(note.tags.is_empty());
}

#[test]
fn structured_note_without_tags_field_defaults_to_empty() {
    let note: JournalNote =
        serde_json::from_str(r#"{"open":"thesis","close":"reflection"}"#).expect("structured note");
    assert_eq!(note.open, "thesis");
    assert_eq!(note.close, "reflection");
    assert!(note.cause_of_error.is_empty());
    assert!(note.tags.is_empty());
}

#[test]
fn note_with_tags_and_cause_round_trips_and_omits_empty_fields() {
    let note = JournalNote {
        open: "thesis".to_string(),
        close: String::new(),
        cause_of_error: "chased late entry".to_string(),
        tags: vec!["breakout".to_string(), "momentum".to_string()],
    };
    let encoded = serde_json::to_string(&note).expect("encode note");
    assert!(encoded.contains("\"cause_of_error\""));
    assert!(encoded.contains("\"tags\""));
    let decoded: JournalNote = serde_json::from_str(&encoded).expect("decode note");
    assert_eq!(decoded.cause_of_error, note.cause_of_error);
    assert_eq!(decoded.tags, note.tags);

    let empty = JournalNote::default();
    let encoded_empty = serde_json::to_string(&empty).expect("encode empty note");
    assert!(!encoded_empty.contains("cause_of_error"));
    assert!(!encoded_empty.contains("tags"));
}

#[test]
fn cause_of_error_counts_as_note_content() {
    let note = JournalNote {
        cause_of_error: "ignored invalidation".to_string(),
        ..Default::default()
    };

    assert!(!note.is_empty());
}

#[test]
fn parse_journal_tags_strips_hashes_and_dedupes() {
    let tags = parse_journal_tags("#breakout, momentum  breakout #Trend");
    assert_eq!(tags, vec!["breakout", "momentum", "Trend"]);
    assert_eq!(journal_tags_input(&tags), "breakout momentum Trend");
    assert!(parse_journal_tags("   #  , ").is_empty());
}
