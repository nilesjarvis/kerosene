use super::{OrderKind, TradingTerminal};

#[test]
fn limit_ioc_reference_price_uses_order_price_not_market_mid() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "BTC".to_string();
    terminal.order_kind = OrderKind::LimitIoc;
    terminal.order_price = "99.5".to_string();
    terminal.all_mids.insert("BTC".to_string(), 101.25);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    assert_eq!(terminal.order_reference_price(), Some(99.5));
}
