use super::*;

#[test]
fn validate_exchange_symbol_orderable_rejects_fallback_outcome() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![outcome_symbol("#660", true)];
    let error = orderability_error(&terminal, first_symbol(&terminal));

    assert!(error.contains("not a tradable market"));
}

#[test]
fn validate_exchange_symbol_orderable_rejects_outcome_without_metadata() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("#650", MarketType::Outcome)];
    let error = orderability_error(&terminal, first_symbol(&terminal));

    assert!(error.contains("metadata is incomplete"));
}
