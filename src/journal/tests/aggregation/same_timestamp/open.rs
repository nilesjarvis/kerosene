use super::{aggregate_trades_with_diagnostics, assert_approx_eq, wallet_hype_fill};

#[test]
fn aggregate_trades_chains_same_timestamp_open_fills_by_position() {
    let time = 1_778_497_097_655;
    let fills = vec![
        wallet_hype_fill(
            time,
            1_055_837_673_236_715,
            "B",
            "Open Long",
            "36.14",
            "0.0",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            679_973_859_119_944,
            "B",
            "Open Long",
            "36.14",
            "36.14",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            92_397_404_714_723,
            "B",
            "Open Long",
            "36.14",
            "72.28",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            848_850_867_302_117,
            "B",
            "Open Long",
            "36.14",
            "108.42",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            608_117_862_803_864,
            "B",
            "Open Long",
            "36.14",
            "144.56",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            218_418_431_393_666,
            "B",
            "Open Long",
            "60.3",
            "180.7",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            59_376_519_919_481,
            "B",
            "Open Long",
            "36.14",
            "241.0",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            646_991_547_874_007,
            "B",
            "Open Long",
            "60.3",
            "277.14",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            1_075_325_548_837_610,
            "B",
            "Open Long",
            "60.3",
            "337.44",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            177_456_786_002_999,
            "B",
            "Open Long",
            "24.26",
            "397.74",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            323_525_496_356_845,
            "B",
            "Open Long",
            "36.14",
            "422.0",
            "0.0",
        ),
        wallet_hype_fill(
            time,
            840_459_077_386_888,
            "B",
            "Open Long",
            "41.86",
            "458.14",
            "0.0",
        ),
    ];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.diagnostics.incomplete_trade_count, 0);
    assert_eq!(result.diagnostics.same_timestamp_position_mismatch_count, 0);
    assert_eq!(result.trades.len(), 1);
    let trade = &result.trades[0];
    assert_eq!(trade.status, "OPEN");
    assert!(trade.is_long);
    assert!(trade.basis_complete);
    assert_eq!(trade.fill_count, 12);
    assert_approx_eq(trade.max_position, 500.0);
    assert_approx_eq(trade.total_entry_size, 500.0);
}
