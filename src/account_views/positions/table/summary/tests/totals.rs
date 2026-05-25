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
fn optional_total_ignores_nonfinite_values() {
    let mut total = OptionalTotal::default();
    total.add(Some(12.0));
    total.add(Some(f64::NAN));
    total.add(Some(f64::INFINITY));

    assert_eq!(total.value(), Some(12.0));
}

#[test]
fn total_pnl_percent_uses_overall_account_balance() {
    let mut total = OptionalTotal::default();
    total.add(Some(50.0));

    assert_eq!(position_total_pnl_percent(total, Some(1_000.0)), Some(5.0));
    assert_eq!(position_total_pnl_percent(total, Some(0.0)), None);
    assert_eq!(position_total_pnl_percent(total, None), None);
}
