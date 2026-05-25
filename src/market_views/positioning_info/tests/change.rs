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
fn positioning_change_sort_defaults_to_largest_absolute_change() {
    let rows = vec![
        delta("0xaaa", 100.0, -5.0),
        delta("0xbbb", 10.0, 50.0),
        delta("0xccc", -10.0, -75.0),
    ];

    let sorted = sorted_change_rows(
        &rows,
        PositioningInfoChangeSortField::Change,
        config::SortDirection::Descending,
        Some(10.0),
    );

    assert_eq!(sorted[0].address, "0xccc");
    assert_eq!(sorted[1].address, "0xbbb");
    assert_eq!(sorted[2].address, "0xaaa");
}

#[test]
fn positioning_change_sort_can_use_derived_previous_size() {
    let rows = vec![
        delta("0xaaa", 0.0, -10.0),
        delta("0xbbb", 30.0, 5.0),
        delta("0xccc", -20.0, 5.0),
    ];

    let sorted = sorted_change_rows(
        &rows,
        PositioningInfoChangeSortField::Previous,
        config::SortDirection::Descending,
        Some(10.0),
    );

    assert_eq!(sorted[0].address, "0xbbb");
    assert_eq!(sorted[1].address, "0xaaa");
    assert_eq!(sorted[2].address, "0xccc");
}

#[test]
fn positioning_change_sort_keeps_invalid_values_last() {
    let rows = vec![
        delta("0xaaa", f64::NAN, 1.0),
        delta("0xbbb", 5.0, 1.0),
        delta("0xccc", 10.0, 1.0),
    ];

    let descending = sorted_change_rows(
        &rows,
        PositioningInfoChangeSortField::Current,
        config::SortDirection::Descending,
        Some(10.0),
    );
    let ascending = sorted_change_rows(
        &rows,
        PositioningInfoChangeSortField::Current,
        config::SortDirection::Ascending,
        Some(10.0),
    );

    assert_eq!(descending[0].address, "0xccc");
    assert_eq!(descending[2].address, "0xaaa");
    assert_eq!(ascending[0].address, "0xbbb");
    assert_eq!(ascending[2].address, "0xaaa");
}
