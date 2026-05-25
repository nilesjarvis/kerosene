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
