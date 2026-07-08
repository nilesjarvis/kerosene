use super::*;

#[test]
fn summary_splits_exposure_and_sums_signed_totals() {
    let mut totals = PositionSummaryTotals::default();

    totals.add_position(Some(true), Some(100.0), Some(2.5), Some(10.0), Some(12.5));
    totals.add_position(Some(false), Some(80.0), Some(-1.0), Some(-3.0), Some(-4.0));

    assert_eq!(totals.long_notional, 100.0);
    assert_eq!(totals.short_notional, 80.0);
    assert_eq!(totals.funding_gross.value(), Some(3.5));
    assert_eq!(totals.net_funding.value(), Some(1.5));
    assert_eq!(totals.upnl.value(), Some(7.0));
    assert_eq!(totals.total_pnl.value(), Some(8.5));
}

#[test]
fn pnl_totals_are_unavailable_when_any_row_is_missing_pnl() {
    let mut totals = PositionSummaryTotals::default();

    totals.add_position(Some(true), Some(100.0), Some(2.5), Some(10.0), Some(12.5));
    // A spot position whose fill-derived cost basis is momentarily
    // unavailable must blank the PnL totals instead of letting them render a
    // partial sum that is wrong by the missing position's PnL.
    totals.add_position(Some(true), Some(80.0), None, None, None);

    assert_eq!(totals.upnl.value(), None);
    assert_eq!(totals.total_pnl.value(), None);
    assert_eq!(totals.funding_gross.value(), Some(2.5));
    assert_eq!(totals.net_funding.value(), Some(2.5));
    assert_eq!(totals.long_notional, 180.0);
}

#[test]
fn optional_total_ignores_nonfinite_values() {
    let mut total = OptionalTotal::default();
    total.add(Some(12.0));
    total.add(Some(f64::NAN));
    total.add(Some(f64::INFINITY));

    assert_eq!(total.value(), Some(12.0));
}

#[test]
fn complete_total_treats_nonfinite_values_as_unknown() {
    let mut total = CompleteTotal::default();
    total.add(Some(12.0));

    assert_eq!(total.value(), Some(12.0));

    total.add(Some(f64::NAN));

    assert_eq!(total.value(), None);
}

#[test]
fn total_pnl_percent_uses_overall_account_balance() {
    assert_eq!(
        position_total_pnl_percent(Some(50.0), Some(1_000.0)),
        Some(5.0)
    );
    assert_eq!(position_total_pnl_percent(Some(50.0), Some(0.0)), None);
    assert_eq!(position_total_pnl_percent(Some(50.0), None), None);
}
