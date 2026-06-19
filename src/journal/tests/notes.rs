use super::fill;
use crate::journal::note_key_for_trade;
use crate::journal::{
    JournalNote, aggregate_trades_with_diagnostics, journal_tags_input, note_for_trade,
    parse_journal_tags,
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
    assert_eq!(note_key_for_trade(&entries, trade), Some(legacy_key));
    assert_eq!(
        note_for_trade(&entries, trade).map(|note| note.open.as_str()),
        Some("legacy note")
    );
}

#[test]
fn legacy_string_note_deserializes_without_tags() {
    let note: JournalNote = serde_json::from_str("\"just text\"").expect("legacy note");
    assert_eq!(note.open, "just text");
    assert!(note.close.is_empty());
    assert!(note.tags.is_empty());
}

#[test]
fn structured_note_without_tags_field_defaults_to_empty() {
    let note: JournalNote =
        serde_json::from_str(r#"{"open":"thesis","close":"reflection"}"#).expect("structured note");
    assert_eq!(note.open, "thesis");
    assert_eq!(note.close, "reflection");
    assert!(note.tags.is_empty());
}

#[test]
fn note_with_tags_round_trips_and_omits_empty_tags() {
    let note = JournalNote {
        open: "thesis".to_string(),
        close: String::new(),
        tags: vec!["breakout".to_string(), "momentum".to_string()],
    };
    let encoded = serde_json::to_string(&note).expect("encode note");
    assert!(encoded.contains("\"tags\""));
    let decoded: JournalNote = serde_json::from_str(&encoded).expect("decode note");
    assert_eq!(decoded.tags, note.tags);

    let empty = JournalNote::default();
    let encoded_empty = serde_json::to_string(&empty).expect("encode empty note");
    assert!(!encoded_empty.contains("tags"));
}

#[test]
fn parse_journal_tags_strips_hashes_and_dedupes() {
    let tags = parse_journal_tags("#breakout, momentum  breakout #Trend");
    assert_eq!(tags, vec!["breakout", "momentum", "Trend"]);
    assert_eq!(journal_tags_input(&tags), "breakout momentum Trend");
    assert!(parse_journal_tags("   #  , ").is_empty());
}
