use super::*;

#[test]
fn outcome_balance_coin_maps_to_trade_coin() {
    assert_eq!(
        TradingTerminal::outcome_balance_coin_to_trade_coin("+650"),
        Some("#650".to_string())
    );
    assert_eq!(
        TradingTerminal::outcome_balance_coin_to_trade_coin("#650"),
        None
    );
}

#[test]
fn display_coin_for_spot_balance_uses_spot_pair_display() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![spot_symbol("@107", "HYPE", "HYPE/USDC")];

    assert_eq!(terminal.display_coin_for_spot_balance("@107"), "HYPE/USDC");
    assert_eq!(terminal.display_coin_for_spot_balance("HYPE"), "HYPE");
}

#[test]
fn display_name_for_symbol_falls_back_to_cached_outcome_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.outcome_display_labels.insert(
        "#650".to_string(),
        "YES: BTC at or above 76,886".to_string(),
    );

    // Trade coin and balance coin both resolve through the cache once the
    // market has expired out of exchange_symbols.
    assert_eq!(
        terminal.display_name_for_symbol("#650"),
        "YES: BTC at or above 76,886"
    );
    assert_eq!(
        terminal.display_name_for_symbol("+650"),
        "YES: BTC at or above 76,886"
    );
    assert_eq!(terminal.display_name_for_symbol("#999"), "#999");
}

#[test]
fn display_coin_for_spot_balance_labels_expired_outcome_via_cache() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.outcome_display_labels.insert(
        "#650".to_string(),
        "YES: BTC at or above 76,886".to_string(),
    );

    assert_eq!(
        terminal.display_coin_for_spot_balance("+650"),
        "YES: BTC at or above 76,886 (+650)"
    );
    assert_eq!(terminal.display_coin_for_spot_balance("+999"), "+999");
}
