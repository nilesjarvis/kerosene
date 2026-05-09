use super::fixtures::{
    ADDRESS_A, ADDRESS_B, labeled_address_book, labeled_address_book_with_color_only,
};
use crate::app_state::TradingTerminal;
use crate::config::{self, AddressBookEntryConfig};
use crate::wallet_state::{AddressBookEntry, address_book::WalletLabelsImportSummary};
use std::collections::HashMap;

#[test]
fn label_export_filters_empty_entries_and_sorts_by_address() {
    let mut address_book = labeled_address_book();
    address_book.insert(
        "0xcccccccccccccccccccccccccccccccccccccccc".to_string(),
        AddressBookEntry::default(),
    );

    let exported = TradingTerminal::wallet_label_export_entries_from_address_book(&address_book);

    assert_eq!(exported.len(), 2);
    assert_eq!(exported[0].address, ADDRESS_A);
    assert_eq!(exported[0].label, "Alpha");
    assert_eq!(exported[1].address, ADDRESS_B);
    assert_eq!(exported[1].label, "Beta");
}

#[test]
fn tracked_trade_subscription_addresses_include_all_labels_only() {
    let address_book = labeled_address_book_with_color_only();

    let addresses = TradingTerminal::labeled_wallet_addresses_from_address_book(&address_book);

    assert_eq!(
        addresses,
        vec![ADDRESS_A.to_string(), ADDRESS_B.to_string()]
    );
}

#[test]
fn label_import_merges_missing_fields_without_overwriting() {
    let mut address_book = HashMap::new();
    address_book.insert(
        ADDRESS_A.to_string(),
        AddressBookEntry {
            label: "Local Label".to_string(),
            color: None,
            tags: vec!["local".to_string()],
        },
    );

    let export = config::WalletLabelsExport {
        schema: config::WALLET_LABELS_EXPORT_SCHEMA.to_string(),
        exported_at_ms: 1,
        labels: vec![
            AddressBookEntryConfig {
                address: ADDRESS_A.to_uppercase(),
                label: "Imported Label".to_string(),
                color: Some("#FF7A1A".to_string()),
                tags: vec!["local".to_string(), "vip".to_string()],
            },
            AddressBookEntryConfig {
                address: ADDRESS_B.to_string(),
                label: "New Label".to_string(),
                color: None,
                tags: Vec::new(),
            },
        ],
    };

    let summary = TradingTerminal::merge_wallet_label_export(&mut address_book, export)
        .expect("valid export should merge");

    assert_eq!(
        summary,
        WalletLabelsImportSummary {
            added: 1,
            updated: 1,
            preserved: 0,
            skipped_invalid: 0,
            skipped_empty: 0,
        }
    );

    let existing = address_book
        .get(ADDRESS_A)
        .expect("existing address remains");
    assert_eq!(existing.label, "Local Label");
    assert_eq!(existing.color.as_deref(), Some("#FF7A1A"));
    assert_eq!(existing.tags, vec!["local".to_string(), "vip".to_string()]);
    assert_eq!(
        address_book
            .get(ADDRESS_B)
            .map(|entry| entry.label.as_str()),
        Some("New Label")
    );
}

#[test]
fn label_import_rejects_unknown_schema() {
    let mut address_book = HashMap::new();
    let export = config::WalletLabelsExport {
        schema: "other.schema".to_string(),
        exported_at_ms: 1,
        labels: Vec::new(),
    };

    let result = TradingTerminal::merge_wallet_label_export(&mut address_book, export);

    assert!(result.is_err());
    assert!(address_book.is_empty());
}
