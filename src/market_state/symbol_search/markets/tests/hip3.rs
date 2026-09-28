use super::*;

#[test]
fn hip3_dexes_are_sorted_unique_perp_prefixes_only() {
    let symbols = vec![
        symbol("xyz:NVDA", MarketType::Perp),
        symbol("@1", MarketType::Spot),
        symbol("abc:BTC", MarketType::Perp),
        symbol("xyz:TSLA", MarketType::Perp),
        symbol("BTC", MarketType::Perp),
    ];

    assert_eq!(
        symbol_search_hip3_dexes(&symbols),
        vec!["abc".to_string(), "xyz".to_string()]
    );
}

#[test]
fn hip3_choices_and_labels_preserve_literal_prefixes() {
    let symbols = [
        symbol("xyz:BTC", MarketType::Perp),
        symbol("XYZ:BTC", MarketType::Perp),
        symbol(":BTC", MarketType::Perp),
        symbol(" a :BTC:USD", MarketType::Perp),
        symbol("xyz:ETH", MarketType::Perp),
        symbol("ignored:AAA", MarketType::Spot),
    ];
    assert_eq!(
        symbol_search_hip3_dexes(&symbols),
        ["", " a ", "XYZ", "xyz"]
    );
    assert_eq!(symbol_search_exchange_label(&symbols[2]), "HIP-3: ");
    assert_eq!(symbol_search_exchange_label(&symbols[3]), "HIP-3:  a ");
    assert_eq!(symbol_search_exchange_label(&symbols[5]), "Spot");
}
