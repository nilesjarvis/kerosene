use super::fixtures::{ADDRESS_A, ADDRESS_B, ADDRESS_C, labeled_address_book_with_color_only};
use crate::app_state::TradingTerminal;
use crate::config::{TrackedWalletConfig, WalletTrackerConfig};
use crate::wallet_state::WalletTrackerState;

#[test]
fn tracker_config_loads_union_of_current_and_legacy_addresses() {
    let cfg = WalletTrackerConfig {
        tracked_addresses: vec![ADDRESS_A.to_string(), ADDRESS_B.to_uppercase()],
        wallets: vec![
            TrackedWalletConfig {
                address: ADDRESS_B.to_string(),
                label: "Duplicate".to_string(),
            },
            TrackedWalletConfig {
                address: ADDRESS_C.to_string(),
                label: "Legacy".to_string(),
            },
        ],
        ..WalletTrackerConfig::default()
    };

    let tracker = WalletTrackerState::from_config(&cfg);

    assert_eq!(
        tracker.tracked_addresses,
        vec![
            ADDRESS_A.to_string(),
            ADDRESS_B.to_string(),
            ADDRESS_C.to_string(),
        ]
    );
}

#[test]
fn labeled_address_book_entries_are_added_to_wallet_tracker_list() {
    let mut tracked_addresses = vec![ADDRESS_B.to_string()];
    let address_book = labeled_address_book_with_color_only();

    let added = TradingTerminal::add_labeled_addresses_to_wallet_tracker(
        &mut tracked_addresses,
        &address_book,
    );

    assert_eq!(added, vec![ADDRESS_A.to_string()]);
    assert_eq!(
        tracked_addresses,
        vec![ADDRESS_B.to_string(), ADDRESS_A.to_string()]
    );
}
