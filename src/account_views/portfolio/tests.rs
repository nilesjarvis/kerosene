use super::totals::{
    format_signed_percent_value, portfolio_total_performance, portfolio_total_pnl,
};

#[test]
fn portfolio_total_pnl_is_unknown_without_points_or_invalid_values() {
    assert_eq!(portfolio_total_pnl(&[]), None);
    assert_eq!(portfolio_total_pnl(&[(1, f64::NAN)]), None);
    assert_eq!(portfolio_total_pnl(&[(1, 1.0), (2, f64::INFINITY)]), None);
}

#[test]
fn portfolio_total_pnl_uses_single_value_or_first_last_delta() {
    assert_eq!(portfolio_total_pnl(&[(1, 5.0)]), Some(5.0));
    assert_eq!(portfolio_total_pnl(&[(1, 5.0), (2, 12.0)]), Some(7.0));
}

#[test]
fn portfolio_total_performance_uses_latest_percent_value() {
    assert_eq!(
        portfolio_total_performance(&[(1, 0.0), (2, 1.5)]),
        Some(1.5)
    );
    assert_eq!(portfolio_total_performance(&[(1, f64::NAN)]), None);
    assert_eq!(
        portfolio_total_performance(&[(1, 1.5), (2, f64::INFINITY)]),
        None
    );
}

#[test]
fn format_signed_percent_value_marks_positive_values() {
    assert_eq!(format_signed_percent_value(1.234), "+1.23%");
    assert_eq!(format_signed_percent_value(-1.234), "-1.23%");
    assert_eq!(format_signed_percent_value(0.001), "0.00%");
}
