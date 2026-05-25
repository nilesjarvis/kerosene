use super::{
    JournalTradeOutcome, account_value_points_for_range, finite_sorted_points,
    format_signed_usd_full, journal_cumulative_pnl_points, journal_recent_trade_outcome_tiles,
};
use crate::journal::AggregatedTrade;

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

    let points = journal_cumulative_pnl_points(&trades);
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
fn recent_trade_outcomes_sort_filter_and_cap_results() {
    let ignored_open = AggregatedTrade {
        status: "OPEN".to_string(),
        ..trade(500, Some(500), 50.0)
    };
    let first = trade(1_000, Some(1_000), 1.0);
    let second = trade(2_000, Some(2_000), -1.0);
    let third = trade(3_000, Some(3_000), 0.0);
    let trades = vec![&third, &ignored_open, &second, &first];

    let outcomes = journal_recent_trade_outcome_tiles(&trades)
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
