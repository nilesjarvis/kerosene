use super::{
    JournalPortfolioPnlKind, JournalTradeOutcome, account_value_points_for_range,
    apply_journal_portfolio_window, finite_sorted_points, format_signed_usd_full,
    journal_all_time_portfolio_pnl_bucket_key, journal_cumulative_pnl_points,
    journal_direct_portfolio_pnl_bucket_key, journal_filter_label, journal_portfolio_pnl_kind,
    journal_recent_trade_outcome_tiles, journal_window_total_pnl, subtract_latest_pnl_series,
};
use crate::journal::{AggregatedTrade, JournalFilter};
use crate::portfolio_state::PortfolioWindow;

fn trade(start_time: u64, end_time: Option<u64>, pnl: f64) -> AggregatedTrade {
    AggregatedTrade {
        id: format!("trade-{start_time}-{pnl}"),
        legacy_note_ids: Vec::new(),
        coin: "BTC".to_string(),
        start_time,
        end_time,
        max_position: 1.0,
        volume: 100.0,
        fee: 1.0,
        pnl,
        status: "CLOSED".to_string(),
        fill_count: 2,
        avg_entry_price: 100.0,
        total_entry_notional: 100.0,
        total_entry_size: 1.0,
        is_long: true,
        basis_complete: true,
    }
}

#[test]
fn cumulative_pnl_points_sort_and_coalesce_trade_times() {
    let first = trade(1_000, Some(3_000), 10.0);
    let second = trade(2_000, Some(2_000), -4.0);
    let third = trade(4_000, Some(3_000), 2.5);
    let trades = vec![&first, &second, &third];

    let points = journal_cumulative_pnl_points(&trades, false);
    assert!(points.len() > 3);
    assert!(points.windows(2).all(|window| window[0].0 < window[1].0));
    assert!(
        points[..points.len() - 2]
            .iter()
            .all(|(_, pnl)| *pnl == 0.0)
    );
    assert!(points.ends_with(&[(2_000, -4.0), (3_000, 8.5)]));
}

#[test]
fn cumulative_pnl_points_apply_fees_when_requested() {
    let first = trade(1_000, Some(1_000), 10.0);
    let second = trade(2_000, Some(2_000), -4.0);
    let trades = vec![&first, &second];

    let points = journal_cumulative_pnl_points(&trades, true);

    assert!(points.ends_with(&[(1_000, 9.0), (2_000, 4.0)]));
}

#[test]
fn portfolio_margin_journal_pnl_kind_maps_spot_to_non_perp() {
    assert_eq!(
        journal_portfolio_pnl_kind(JournalFilter::All),
        Some(JournalPortfolioPnlKind::All)
    );
    assert_eq!(
        journal_portfolio_pnl_kind(JournalFilter::Perp),
        Some(JournalPortfolioPnlKind::Perp)
    );
    assert_eq!(
        journal_portfolio_pnl_kind(JournalFilter::Spot),
        Some(JournalPortfolioPnlKind::NonPerp)
    );
    assert_eq!(journal_portfolio_pnl_kind(JournalFilter::Outcome), None);
}

#[test]
fn portfolio_margin_journal_bucket_keys_follow_selected_window() {
    assert_eq!(
        journal_direct_portfolio_pnl_bucket_key(JournalPortfolioPnlKind::All, PortfolioWindow::Day),
        Some("day")
    );
    assert_eq!(
        journal_direct_portfolio_pnl_bucket_key(
            JournalPortfolioPnlKind::Perp,
            PortfolioWindow::Week
        ),
        Some("perpWeek")
    );
    assert_eq!(
        journal_direct_portfolio_pnl_bucket_key(JournalPortfolioPnlKind::All, PortfolioWindow::Mtd),
        None
    );
    assert_eq!(
        journal_all_time_portfolio_pnl_bucket_key(JournalPortfolioPnlKind::All),
        Some("allTime")
    );
    assert_eq!(
        journal_all_time_portfolio_pnl_bucket_key(JournalPortfolioPnlKind::Perp),
        Some("perpAllTime")
    );
    assert_eq!(
        journal_all_time_portfolio_pnl_bucket_key(JournalPortfolioPnlKind::NonPerp),
        None
    );
}

#[test]
fn portfolio_margin_spot_pnl_uses_all_minus_perp_history() {
    let all_points = vec![(3_000, 150.0), (1_000, 0.0), (2_000, 100.0)];
    let perp_points = vec![(1_000, 0.0), (2_000, 25.0), (4_000, 50.0)];

    let points = subtract_latest_pnl_series(&all_points, &perp_points);

    assert_eq!(points, vec![(1_000, 0.0), (2_000, 75.0), (3_000, 125.0)]);
    assert_eq!(journal_window_total_pnl(&points), Some(125.0));
}

#[test]
fn account_value_points_are_clamped_to_pnl_time_range() {
    let points = vec![
        (1_000, 90.0),
        (2_000, 100.0),
        (3_000, 110.0),
        (4_000, 120.0),
    ];

    assert_eq!(
        account_value_points_for_range(&points, 2_500, 3_500),
        vec![(2_500, 100.0), (3_000, 110.0)]
    );
}

#[test]
fn chart_points_are_sorted_and_nonfinite_values_are_skipped() {
    assert_eq!(
        finite_sorted_points(&[(3_000, 3.0), (1_000, f64::NAN), (2_000, 2.0)]),
        vec![(2_000, 2.0), (3_000, 3.0)]
    );
}

#[test]
fn signed_usd_full_keeps_large_values_expanded() {
    assert_eq!(format_signed_usd_full(29_425_659.43), "+$29,425,659.43");
    assert_eq!(format_signed_usd_full(-42.5), "-$42.50");
    assert_eq!(format_signed_usd_full(0.001), "$0.00");
}

#[test]
fn journal_portfolio_window_applies_cutoff_with_baseline() {
    let day_ms = 24 * 60 * 60 * 1000;
    let now_ms = 10 * day_ms;
    let points = vec![(day_ms, 5.0), (8 * day_ms, 9.0), (9 * day_ms, 12.0)];

    let windowed = apply_journal_portfolio_window(points, PortfolioWindow::Week, now_ms);

    assert_eq!(
        windowed,
        vec![(3 * day_ms, 5.0), (8 * day_ms, 9.0), (9 * day_ms, 12.0)]
    );
    assert_eq!(journal_window_total_pnl(&windowed), Some(7.0));
}

#[test]
fn journal_portfolio_all_time_keeps_all_points() {
    let points = vec![(1_000, 5.0), (2_000, 9.0)];

    assert_eq!(
        apply_journal_portfolio_window(points.clone(), PortfolioWindow::AllTime, 3_000),
        points
    );
}

#[test]
fn journal_filter_labels_include_outcome_filter() {
    assert_eq!(journal_filter_label(JournalFilter::All), "All");
    assert_eq!(journal_filter_label(JournalFilter::Perp), "Perp");
    assert_eq!(journal_filter_label(JournalFilter::Spot), "Spot");
    assert_eq!(journal_filter_label(JournalFilter::Outcome), "Outcome");
}

#[test]
fn recent_trade_outcomes_sort_filter_and_cap_results() {
    let ignored_open = AggregatedTrade {
        status: "OPEN".to_string(),
        ..trade(500, Some(500), 50.0)
    };
    let first = trade(1_000, Some(1_000), 1.0);
    let second = trade(2_000, Some(2_000), -1.0);
    let third = trade(3_000, Some(3_000), 0.0);
    let trades = vec![&third, &ignored_open, &second, &first];

    let outcomes = journal_recent_trade_outcome_tiles(&trades, false)
        .into_iter()
        .map(|tile| (tile.outcome, tile.trade_type, tile.pnl))
        .collect::<Vec<_>>();

    assert_eq!(
        outcomes,
        vec![
            (JournalTradeOutcome::Win, "Long BTC".to_string(), 1.0),
            (JournalTradeOutcome::Loss, "Long BTC".to_string(), -1.0),
            (JournalTradeOutcome::Flat, "Long BTC".to_string(), 0.0),
        ]
    );
}

#[test]
fn recent_trade_outcomes_apply_fees_when_requested() {
    let win_after_fees = trade(1_000, Some(1_000), 2.0);
    let flat_after_fees = trade(2_000, Some(2_000), 1.0);
    let loss_after_fees = trade(3_000, Some(3_000), 0.5);
    let trades = vec![&loss_after_fees, &flat_after_fees, &win_after_fees];

    let outcomes = journal_recent_trade_outcome_tiles(&trades, true)
        .into_iter()
        .map(|tile| (tile.outcome, tile.pnl))
        .collect::<Vec<_>>();

    assert_eq!(
        outcomes,
        vec![
            (JournalTradeOutcome::Win, 1.0),
            (JournalTradeOutcome::Flat, 0.0),
            (JournalTradeOutcome::Loss, -0.5),
        ]
    );
}
