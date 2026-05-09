use crate::wallet_state::AddressBookEntry;
use std::collections::HashMap;

pub(super) const ADDRESS_A: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(super) const ADDRESS_B: &str = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(super) const ADDRESS_C: &str = "0xcccccccccccccccccccccccccccccccccccccccc";
pub(super) const LIQUIDATION_ADDRESS: &str = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

pub(super) fn labeled_address_book() -> HashMap<String, AddressBookEntry> {
    HashMap::from([
        (
            ADDRESS_B.to_string(),
            AddressBookEntry {
                label: "Beta".to_string(),
                ..Default::default()
            },
        ),
        (
            ADDRESS_A.to_string(),
            AddressBookEntry {
                label: "Alpha".to_string(),
                ..Default::default()
            },
        ),
    ])
}

pub(super) fn labeled_address_book_with_color_only() -> HashMap<String, AddressBookEntry> {
    let mut address_book = labeled_address_book();
    address_book.insert(
        ADDRESS_C.to_string(),
        AddressBookEntry {
            color: Some("#FF7A1A".to_string()),
            ..Default::default()
        },
    );
    address_book
}
