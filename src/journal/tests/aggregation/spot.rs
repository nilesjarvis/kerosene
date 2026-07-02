use super::{aggregate_trades_with_diagnostics, assert_approx_eq, spot_fill};

// Spot trades are bucketed per order id and carry realized PnL on closing (sell)
// fills plus fees that may be denominated in a non-USDC token. These tests pin the
// aggregation of both, which were previously dropped (every spot trade showed PnL 0
// and base-token fees were summed as if they were USD).

#[test]
fn spot_sell_aggregates_realized_pnl_from_closed_pnl() {
    let fills = vec![spot_fill(
        1_000, 1, 9_001, "@107", "A", "40.0", "2.0", "0.5", "USDC", "25.0",
    )];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.trades.len(), 1);
    let trade = &result.trades[0];
    assert_eq!(trade.coin, "@107");
    assert_eq!(trade.status, "FILLED");
    assert_approx_eq(trade.pnl, 25.0);
    assert_approx_eq(trade.fee, 0.5);
    assert_approx_eq(trade.max_position, -2.0);
    assert_approx_eq(trade.volume, 80.0);
}

#[test]
fn spot_buy_fee_in_base_token_is_converted_to_usd() {
    // A HYPE/USDC buy is charged its fee in HYPE (0.05 HYPE at px 40 USDC = 2.00 USD).
    let fills = vec![spot_fill(
        1_000, 1, 9_002, "@107", "B", "40.0", "1.0", "0.05", "HYPE", "0.0",
    )];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.trades.len(), 1);
    let trade = &result.trades[0];
    assert_approx_eq(trade.fee, 2.0);

    let details = result
        .trade_details
        .get(&trade.id)
        .expect("spot trade details");
    assert_eq!(details.attributed_fills.len(), 1);
    assert_approx_eq(details.attributed_fills[0].fee, 2.0);
}

#[test]
fn spot_usdc_fee_is_left_unchanged() {
    let fills = vec![spot_fill(
        1_000, 1, 9_003, "@107", "B", "40.0", "1.0", "0.5", "USDC", "0.0",
    )];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.trades.len(), 1);
    assert_approx_eq(result.trades[0].fee, 0.5);
    assert_approx_eq(result.trades[0].pnl, 0.0);
}

#[test]
fn spot_sell_fee_in_non_usdc_stable_quote_is_not_scaled_by_price() {
    // Selling 0.1 UBTC on a USDT0-quoted pair at px 84,000 pays its fee in the
    // quote token: ~4.7 USDT0, a ~$1 stable that is already USD-denominated.
    // Multiplying by px (the old behavior) recorded a ~$394,800 fee.
    let fills = vec![spot_fill(
        1_000, 1, 9_030, "@142", "A", "84000.0", "0.1", "4.7", "USDT0", "150.0",
    )];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.trades.len(), 1);
    let trade = &result.trades[0];
    assert_approx_eq(trade.fee, 4.7);
    assert_approx_eq(trade.pnl, 150.0);

    let details = result
        .trade_details
        .get(&trade.id)
        .expect("spot trade details");
    assert_approx_eq(details.attributed_fills[0].fee, 4.7);
}

#[test]
fn outcome_sell_fee_in_usdh_quote_is_left_unchanged() {
    // Outcome markets quote in USDH with prices in [0, 1]; scaling the quote
    // fee by px would understate it (0.12 USDH * 0.35 = $0.042).
    let fills = vec![spot_fill(
        1_000, 1, 9_031, "#950", "A", "0.35", "100.0", "0.12", "USDH", "5.0",
    )];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.trades.len(), 1);
    assert_approx_eq(result.trades[0].fee, 0.12);
}

#[test]
fn spot_stable_quote_fee_token_matches_case_insensitively() {
    let fills = vec![spot_fill(
        1_000, 1, 9_032, "@142", "A", "84000.0", "0.1", "3.2", "usde", "0.0",
    )];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.trades.len(), 1);
    assert_approx_eq(result.trades[0].fee, 3.2);
}

#[test]
fn spot_buy_and_sell_of_same_token_are_separate_trades_each_with_own_pnl() {
    let fills = vec![
        spot_fill(
            1_000, 1, 9_010, "@107", "B", "40.0", "1.0", "0.04", "HYPE", "0.0",
        ),
        spot_fill(
            2_000, 2, 9_011, "@107", "A", "50.0", "1.0", "0.5", "USDC", "10.0",
        ),
    ];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.trades.len(), 2);

    let buy = result
        .trades
        .iter()
        .find(|trade| trade.id.ends_with(":9010"))
        .expect("buy trade");
    let sell = result
        .trades
        .iter()
        .find(|trade| trade.id.ends_with(":9011"))
        .expect("sell trade");

    assert_approx_eq(buy.pnl, 0.0);
    assert_approx_eq(sell.pnl, 10.0);
    let total_pnl: f64 = result.trades.iter().map(|trade| trade.pnl).sum();
    assert_approx_eq(total_pnl, 10.0);
}

#[test]
fn spot_trades_record_order_side_so_sell_vwap_is_not_an_entry() {
    // One spot trade is one order, so its side must be carried on the trade:
    // a sell's VWAP is its sale price and must not render as ENTRY.
    let fills = vec![
        spot_fill(
            1_000, 1, 9_040, "@107", "B", "40.0", "1.0", "0.0", "USDC", "0.0",
        ),
        spot_fill(
            2_000, 2, 9_041, "@107", "A", "50.0", "1.0", "0.0", "USDC", "10.0",
        ),
    ];

    let result = aggregate_trades_with_diagnostics(fills);

    let buy = result
        .trades
        .iter()
        .find(|trade| trade.id.ends_with(":9040"))
        .expect("buy trade");
    let sell = result
        .trades
        .iter()
        .find(|trade| trade.id.ends_with(":9041"))
        .expect("sell trade");

    assert!(buy.is_long);
    assert_approx_eq(buy.avg_entry_price, 40.0);
    assert!(!sell.is_long);
    assert_approx_eq(sell.avg_entry_price, 50.0);
}

#[test]
fn spot_multi_fill_order_accumulates_pnl_fee_and_fill_count() {
    // Two partial fills of one sell order (same oid) collapse into a single trade.
    let fills = vec![
        spot_fill(
            1_000, 1, 9_020, "@107", "A", "40.0", "1.0", "0.2", "USDC", "10.0",
        ),
        spot_fill(
            1_500, 2, 9_020, "@107", "A", "42.0", "1.0", "0.3", "USDC", "15.0",
        ),
    ];

    let result = aggregate_trades_with_diagnostics(fills);

    assert_eq!(result.trades.len(), 1);
    let trade = &result.trades[0];
    assert_eq!(trade.fill_count, 2);
    assert_approx_eq(trade.pnl, 25.0);
    assert_approx_eq(trade.fee, 0.5);
    assert_approx_eq(trade.max_position, -2.0);
}
