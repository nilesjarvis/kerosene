use super::{aggregate_trades_with_diagnostics, fill};

#[test]
fn aggregate_trades_skips_malformed_numeric_fills() {
    let mut malformed = fill(1, 10, "BTC");
    malformed.sz = "not-a-number".to_string();

    let result = aggregate_trades_with_diagnostics(vec![malformed]);

    assert!(result.trades.is_empty());
    assert_eq!(result.diagnostics.skipped_fill_count, 1);
}

#[test]
fn aggregate_trades_marks_missing_opening_basis_as_partial() {
    let mut close = fill(1, 10, "BTC");
    close.side = "A".to_string();
    close.start_position = "1.0".to_string();
    close.dir = "Close Long".to_string();
    close.closed_pnl = "10.0".to_string();

    let result = aggregate_trades_with_diagnostics(vec![close]);

    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.diagnostics.incomplete_trade_count, 1);
    assert!(!result.trades[0].basis_complete);
    assert_eq!(result.trades[0].pnl, 10.0);
}
