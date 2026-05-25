use super::{aggregate_trades_with_diagnostics, assert_approx_eq, wallet_hype_fill};

#[test]
fn aggregate_trades_keeps_same_timestamp_close_fills_in_the_long_trade() {
    let open_time = 1_778_596_387_586;
    let close_time = 1_778_596_428_000;
    let fills = vec![
        wallet_hype_fill(
            open_time,
            21_400_535_966_404,
            "B",
            "Open Long",
            "1.64",
            "0.0",
            "0.0",
        ),
        wallet_hype_fill(
            open_time + 488,
            232_957_291_404_586,
            "B",
            "Open Long",
            "45.57",
            "1.64",
            "0.0",
        ),
        wallet_hype_fill(
            open_time + 8_540,
            296_712_036_058_506,
            "B",
            "Open Long",
            "452.79",
            "47.21",
            "0.0",
        ),
        wallet_hype_fill(
            close_time - 127,
            420_936_527_112_987,
            "A",
            "Close Long",
            "143.29",
            "500.0",
            "7.30779",
        ),
        wallet_hype_fill(
            close_time,
            364_046_121_164_912,
            "A",
            "Close Long",
            "332.43",
            "356.71",
            "16.95393",
        ),
        wallet_hype_fill(
            close_time,
            276_590_854_497_898,
            "A",
            "Close Long",
            "24.28",
            "24.28",
            "1.23828",
        ),
    ];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.diagnostics.incomplete_trade_count, 0);
    assert_eq!(result.trades.len(), 1);
    let trade = &result.trades[0];
    assert_eq!(trade.status, "CLOSED");
    assert!(trade.is_long);
    assert_eq!(trade.fill_count, 6);
    assert_approx_eq(trade.max_position, 500.0);
    assert_approx_eq(trade.pnl, 25.5);
}
