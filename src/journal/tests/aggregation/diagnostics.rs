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
fn closed_trade_recovers_missing_opening_basis_from_realized_pnl() {
    // Only a closing fill is loaded (the open predates Hyperliquid's history).
    // closedPnl = (exit - entry) * size for a long, so entry = 100 - 10/1 = 90.
    // The trade is recovered as complete with that derived entry, not partial.
    let mut close = fill(1, 10, "BTC");
    close.side = "A".to_string();
    close.start_position = "1.0".to_string();
    close.dir = "Close Long".to_string();
    close.closed_pnl = "10.0".to_string();

    let result = aggregate_trades_with_diagnostics(vec![close]);

    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.diagnostics.incomplete_trade_count, 0);
    assert!(result.trades[0].basis_complete);
    assert!((result.trades[0].avg_entry_price - 90.0).abs() < 1e-9);
    assert_eq!(result.trades[0].pnl, 10.0);
}

#[test]
fn closed_pre_data_entry_is_size_weighted_across_closing_fills() {
    // A carried-in long (true entry 100) closed over two fills at different
    // prices; each closing fill's closedPnl implies entry 100, so the
    // size-weighted recovered entry is exactly 100.
    let mut first = fill(10, 1, "RUNE");
    first.side = "A".to_string();
    first.sz = "2.0".to_string();
    first.px = "110.0".to_string();
    first.start_position = "5.0".to_string();
    first.dir = "Close Long".to_string();
    first.closed_pnl = "20.0".to_string(); // (110 - 100) * 2

    let mut second = fill(20, 2, "RUNE");
    second.side = "A".to_string();
    second.sz = "3.0".to_string();
    second.px = "90.0".to_string();
    second.start_position = "3.0".to_string();
    second.dir = "Close Long".to_string();
    second.closed_pnl = "-30.0".to_string(); // (90 - 100) * 3

    let result = aggregate_trades_with_diagnostics(vec![first, second]);

    assert_eq!(result.trades.len(), 1);
    assert!(result.trades[0].basis_complete);
    assert!((result.trades[0].avg_entry_price - 100.0).abs() < 1e-9);
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
fn slash_named_spot_pair_is_classified_as_spot() {
    // `PURR/USDC` is a Hyperliquid spot pair; it must be aggregated as spot
    // (always complete) rather than tracked as a never-flat perp position.
    let purr = fill(10, 1, "PURR/USDC");

    let result = aggregate_trades_with_diagnostics(vec![purr]);

    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].status, "FILLED");
    assert!(result.trades[0].basis_complete);
    assert!(result.trades[0].id.starts_with("spot:"));
}

#[test]
fn flat_open_coin_pure_reduce_close_recovers_entry_from_pnl() {
    // The coin opened from flat earlier (so the flat-open scan marks later
    // chains complete), but this later chain is a pure carried-in close with no
    // in-window opening fill. Its entry must be recovered from realized PnL
    // rather than left at 0/blank.
    let open = fill(10, 1, "BTC");

    let mut close = fill(20, 2, "BTC");
    close.side = "A".to_string();
    close.start_position = "1.0".to_string();
    close.dir = "Close Long".to_string();

    let mut carried_close = fill(30, 3, "BTC");
    carried_close.side = "A".to_string();
    carried_close.sz = "2.0".to_string();
    carried_close.px = "120.0".to_string();
    carried_close.start_position = "2.0".to_string();
    carried_close.dir = "Close Long".to_string();
    carried_close.closed_pnl = "40.0".to_string(); // (120 - 100) * 2 ⇒ entry 100

    let result = aggregate_trades_with_diagnostics(vec![open, close, carried_close]);

    assert_eq!(result.diagnostics.incomplete_trade_count, 0);
    assert!(
        result
            .trades
            .iter()
            .all(|trade| trade.basis_complete && trade.avg_entry_price > 0.0)
    );
}

#[test]
fn open_from_minimum_lot_residual_is_complete() {
    // An OPEN position that began from a sub-lot dust residual (startPosition at
    // HL's minimum lot, just above the structural 1e-6 epsilon) counts as
    // opened-from-flat, so it is complete even though it never closes (the
    // realized-PnL recovery only applies to closed trades).
    let mut open = fill(10, 1, "ZEC");
    open.start_position = "0.01".to_string();
    open.sz = "5.0".to_string();

    let result = aggregate_trades_with_diagnostics(vec![open]);

    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].status, "OPEN");
    assert!(result.trades[0].basis_complete);
}

#[test]
fn open_carried_in_above_dust_threshold_stays_partial() {
    // An OPEN carried-in position worth far more than the dust notional
    // threshold (0.05 @ $1000 = $50 ≫ $5, no flat open) stays partial: the dust
    // tolerance must not swallow a real position, and realized-PnL recovery does
    // not apply to still-open trades.
    let mut reduce = fill(10, 1, "BCH");
    reduce.side = "A".to_string();
    reduce.sz = "0.03".to_string();
    reduce.px = "1000.0".to_string();
    reduce.start_position = "0.05".to_string();
    reduce.dir = "Close Long".to_string();

    let result = aggregate_trades_with_diagnostics(vec![reduce]);

    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].status, "OPEN");
    assert!(!result.trades[0].basis_complete);
}

#[test]
fn open_chain_with_no_flat_open_in_window_stays_partial() {
    // A genuinely truncated position still OPEN at window end (no flat open, no
    // realized close to recover from) stays partial.
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
