use super::*;

#[test]
fn positioning_change_usd_uses_live_mark() {
    assert_eq!(positioning_live_change_usd(-2.5, Some(20.0)), Some(-50.0));
    assert_eq!(positioning_live_change_usd(2.5, None), None);
    assert_eq!(positioning_live_change_usd(2.5, Some(0.0)), None);
    assert_eq!(positioning_live_change_usd(f64::NAN, Some(20.0)), None);
}

#[test]
fn positioning_change_previous_size_is_derived_from_current_and_delta() {
    assert_eq!(
        positioning_previous_change_size(&delta("0xaaa", 0.0, -50.0)),
        Some(50.0)
    );
    assert_eq!(
        positioning_previous_change_size(&delta("0xbbb", 65.5, 65.5)),
        Some(0.0)
    );
    assert_eq!(
        positioning_previous_change_size(&delta("0xccc", -100.0, 30.0)),
        Some(-130.0)
    );
}

#[test]
fn positioning_change_side_totals_count_flips_by_side_exposure() {
    let rows = vec![
        delta("0xaaa", -5.0, -15.0),
        delta("0xbbb", 2.0, 10.0),
        delta("0xccc", 7.0, 4.0),
        delta("0xddd", f64::NAN, 1.0),
    ];

    let totals = positioning_change_side_delta_totals(&rows);

    assert_eq!(totals.long_delta, -4.0);
    assert_eq!(totals.short_delta, -3.0);
}

#[test]
fn positioning_flow_ranks_by_largest_usd_magnitude() {
    let rows = vec![
        delta("0xaaa", 100.0, -5.0),
        delta("0xbbb", 10.0, 50.0),
        delta("0xccc", -10.0, -75.0),
    ];

    let data = positioning_flow_data(&rows, Some(10.0), 10);

    // Sorted by |delta * mark| descending: 75 > 50 > 5.
    assert_eq!(data.rows[0].address, "0xccc");
    assert_eq!(data.rows[1].address, "0xbbb");
    assert_eq!(data.rows[2].address, "0xaaa");
    assert!(data.usd_scaled);
    assert_eq!(data.max_magnitude, 750.0);
}

#[test]
fn positioning_flow_falls_back_to_size_without_live_mark() {
    let rows = vec![delta("0xaaa", 100.0, -5.0), delta("0xbbb", 10.0, 50.0)];

    let data = positioning_flow_data(&rows, None, 10);

    assert!(!data.usd_scaled);
    // Ranked by raw |delta|: 50 > 5.
    assert_eq!(data.rows[0].address, "0xbbb");
    assert_eq!(data.max_magnitude, 50.0);
}

#[test]
fn positioning_flow_classifies_add_cut_and_flip() {
    // Grew an existing long (prev 95 -> now 100).
    let add = positioning_flow_data(&[delta("0xaaa", 100.0, 5.0)], Some(10.0), 10);
    assert_eq!(add.rows[0].kind, PositioningFlowKind::Add);

    // Shrank a long toward zero (prev 60 -> now 10).
    let cut = positioning_flow_data(&[delta("0xbbb", 10.0, -50.0)], Some(10.0), 10);
    assert_eq!(cut.rows[0].kind, PositioningFlowKind::Cut);

    // Crossed zero short -> long (prev -20 -> now 30).
    let flip = positioning_flow_data(&[delta("0xccc", 30.0, 50.0)], Some(10.0), 10);
    assert_eq!(flip.rows[0].kind, PositioningFlowKind::Flip);
}

#[test]
fn positioning_flow_aggregates_net_long_and_short_flow() {
    let rows = vec![
        delta("0xaaa", 0.0, 4.0),  // +4 long-ward
        delta("0xbbb", 0.0, -3.0), // -3 short-ward
        delta("0xccc", f64::NAN, 1.0),
    ];

    let data = positioning_flow_data(&rows, Some(10.0), 10);

    // Long flow = 4 * 10, short flow = 3 * 10; non-finite current is skipped.
    assert_eq!(data.long_flow, 40.0);
    assert_eq!(data.short_flow, 30.0);
}

#[test]
fn positioning_flow_respects_row_limit() {
    let rows = vec![
        delta("0xaaa", 100.0, -5.0),
        delta("0xbbb", 10.0, 50.0),
        delta("0xccc", -10.0, -75.0),
    ];

    let data = positioning_flow_data(&rows, Some(10.0), 2);

    assert_eq!(data.rows.len(), 2);
    // Keeps the two largest moves.
    assert_eq!(data.rows[0].address, "0xccc");
    assert_eq!(data.rows[1].address, "0xbbb");
}
