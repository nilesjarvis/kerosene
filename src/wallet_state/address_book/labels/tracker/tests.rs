use super::*;
use crate::config::KeroseneConfig;

#[test]
fn combined_labels_sort_and_deduplicate_both_sources_before_muting() {
    let a = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let b = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let c = "0xcccccccccccccccccccccccccccccccccccccccc";
    let d = "0xdddddddddddddddddddddddddddddddddddddddd";
    let e = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    let f = "0xffffffffffffffffffffffffffffffffffffffff";
    let book = |entries: &[(&str, &str)]| {
        entries
            .iter()
            .map(|(address, label)| {
                (
                    address.to_string(),
                    AddressBookEntry {
                        label: label.to_string(),
                        color: Some("#ff00ff".into()),
                        tags: vec!["metadata-alone-does-not-subscribe".into()],
                    },
                )
            })
            .collect()
    };
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.address_book = book(&[(c, "Local only"), (b, ""), (a, "Local"), (e, " \t")]);
    terminal.wallet_tracker.remote_database.entries = book(&[
        (d, "Remote"),
        (c, "\n"),
        (b, "Remote only"),
        (a, "Remote override"),
        (f, ""),
    ]);
    terminal.wallet_tracker.tracked_addresses.clear();
    terminal.wallet_tracker.muted_addresses = vec![a.into(), d.into(), d.into(), "unknown".into()];

    assert_eq!(
        TradingTerminal::labeled_wallet_addresses_from_address_book(&terminal.address_book),
        [a, c]
    );
    assert_eq!(terminal.labeled_wallet_addresses(), [a, b, c, d]);
    assert_eq!(terminal.tracked_trade_subscription_addresses(), [b, c]);
    assert!(terminal.wallet_tracker.tracked_addresses.is_empty());
    let mut returned = terminal.labeled_wallet_addresses();
    returned[0].clear();
    assert_eq!(terminal.labeled_wallet_addresses(), [a, b, c, d]);
    terminal.address_book.clear();
    assert_eq!(terminal.labeled_wallet_addresses(), [a, b, d]);
    assert_eq!(terminal.tracked_trade_subscription_addresses(), [b]);
    terminal.wallet_tracker.remote_database.entries.clear();
    assert!(terminal.labeled_wallet_addresses().is_empty());
    assert!(terminal.tracked_trade_subscription_addresses().is_empty());
}
