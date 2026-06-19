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

#[test]
fn reduce_led_chain_is_complete_when_coin_opened_from_flat_in_window() {
    // The coin opens from flat (basis present) and fully closes, then a later
    // chain begins on a *reducing* fill with a non-flat start position — exactly
    // what same-timestamp fill ordering or a dust residual can produce. Because
    // a `startPosition == 0` open for this coin exists at/before that chain, the
    // trade must NOT be flagged partial (the prior chain-head heuristic did).
    let open = fill(10, 1, "BTC");

    let mut close = fill(20, 2, "BTC");
    close.side = "A".to_string();
    close.sz = "1.0".to_string();
    close.start_position = "1.0".to_string();
    close.dir = "Close Long".to_string();

    let mut reduce = fill(30, 3, "BTC");
    reduce.side = "A".to_string();
    reduce.sz = "0.5".to_string();
    reduce.start_position = "0.5".to_string();
    reduce.dir = "Close Long".to_string();

    let result = aggregate_trades_with_diagnostics(vec![open, close, reduce]);

    assert_eq!(result.diagnostics.incomplete_trade_count, 0);
    assert!(result.trades.iter().all(|trade| trade.basis_complete));
}

#[test]
fn chain_with_no_flat_open_in_window_stays_partial() {
    // A genuinely truncated position: every loaded fill reduces a position that
    // was opened before the loaded history (no `startPosition == 0` anywhere),
    // so the trade is correctly flagged partial.
    let mut first = fill(10, 1, "XPL");
    first.side = "A".to_string();
    first.sz = "100.0".to_string();
    first.start_position = "500.0".to_string();
    first.dir = "Close Long".to_string();

    let mut second = fill(20, 2, "XPL");
    second.side = "A".to_string();
    second.sz = "100.0".to_string();
    second.start_position = "400.0".to_string();
    second.dir = "Close Long".to_string();

    let result = aggregate_trades_with_diagnostics(vec![first, second]);

    assert!(result.diagnostics.incomplete_trade_count >= 1);
    assert!(result.trades.iter().all(|trade| !trade.basis_complete));
}
