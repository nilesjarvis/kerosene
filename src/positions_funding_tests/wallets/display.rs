use super::fixtures::LIQUIDATION_ADDRESS;
use crate::app_state::TradingTerminal;
use crate::wallet_state::AddressBookEntry;
use crate::ws::LiquidationEvent;
use std::collections::HashMap;

#[test]
fn wallet_address_normalization_requires_full_hex_address() {
    assert_eq!(
        TradingTerminal::normalize_wallet_address("  0xEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE  ")
            .as_deref(),
        Some(LIQUIDATION_ADDRESS)
    );
    assert!(TradingTerminal::normalize_wallet_address("0xnot-hex").is_none());
    assert!(
        TradingTerminal::normalize_wallet_address("0x5CBDB794B3B36DF58A7CE6C1A552F117F061103Z")
            .is_none()
    );
    assert!(
        TradingTerminal::normalize_wallet_address("5CBDB794B3B36DF58A7CE6C1A552F117F061103B")
            .is_none()
    );
}

#[test]
fn display_uses_label_for_normalized_liquidation_address() {
    let mut address_book = HashMap::new();
    address_book.insert(
        LIQUIDATION_ADDRESS.to_string(),
        AddressBookEntry {
            label: "Tracked Wallet".to_string(),
            ..Default::default()
        },
    );

    let display = TradingTerminal::wallet_display_from_address_book(
        &address_book,
        "0xEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE",
    );

    assert_eq!(display.primary, "Tracked Wallet");
    assert_eq!(display.secondary, "0xeeee...eeee");
    assert!(display.has_label);
}

#[test]
fn liquidation_events_normalize_liquidated_user_before_storage() {
    let liq = LiquidationEvent {
        coin: "HYPE".to_string(),
        price: 10.0,
        size: 2.0,
        is_buy: true,
        time_ms: 1,
        method: "market".to_string(),
        liquidated_user: "  0xEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE  ".to_string(),
        tx_index: 7,
    };

    let normalized = TradingTerminal::normalize_liquidation_event(liq);

    assert_eq!(normalized.liquidated_user, LIQUIDATION_ADDRESS);
}
