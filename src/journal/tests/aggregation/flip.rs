use super::{aggregate_trades_with_diagnostics, assert_approx_eq, wallet_hype_fill};
use crate::journal::JournalAttributedFillRole;

#[test]
fn aggregate_trades_attributes_flip_fill_to_close_and_open_trades() {
    let fills = vec![
        wallet_hype_fill(1_000, 1, "B", "Open Long", "1.0", "0.0", "0.0"),
        wallet_hype_fill(2_000, 2, "A", "Open Short", "3.0", "1.0", "10.0"),
    ];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.trades.len(), 2);
    let open_short = result
        .trades
        .iter()
        .find(|trade| trade.status == "OPEN")
        .expect("open short trade");
    let closed_long = result
        .trades
        .iter()
        .find(|trade| trade.status == "CLOSED")
        .expect("closed long trade");

    assert!(!open_short.is_long);
    assert!(closed_long.is_long);

    let closed_details = result
        .trade_details
        .get(&closed_long.id)
        .expect("closed details");
    let flip_close = closed_details
        .attributed_fills
        .iter()
        .find(|fill| fill.role == JournalAttributedFillRole::FlipClose)
        .expect("flip close fragment");
    assert_approx_eq(flip_close.attributed_size, 1.0);

    let open_details = result
        .trade_details
        .get(&open_short.id)
        .expect("open details");
    let flip_open = open_details
        .attributed_fills
        .iter()
        .find(|fill| fill.role == JournalAttributedFillRole::FlipOpen)
        .expect("flip open fragment");
    assert_approx_eq(flip_open.attributed_size, 2.0);
}
