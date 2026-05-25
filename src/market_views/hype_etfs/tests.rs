use super::*;
use crate::denomination::DisplayDenominationContext;
use crate::hype_etf_state::HypeEtfDailyFlow;

fn daily_flow(amount_usd: f64) -> HypeEtfDailyFlow {
    HypeEtfDailyFlow {
        date: "2026-05-20".to_string(),
        amount_usd,
    }
}

#[test]
fn cumulative_inflows_tracks_running_total() {
    let flows = vec![daily_flow(100.0), daily_flow(-25.0), daily_flow(10.0)];

    assert_eq!(cumulative_inflows(&flows), vec![100.0, 75.0, 85.0]);
}

#[test]
fn cumulative_line_points_stay_inside_chart_bounds() {
    let values = [100.0, 50.0, 125.0];
    let scale = flow_chart_scale(&values, FLOW_CHART_HEIGHT);
    let points = cumulative_line_points(&values, 300.0, FLOW_CHART_HEIGHT, scale);

    assert_eq!(points.len(), 3);
    assert!(points[0].x < points[1].x);
    assert!(points[1].x < points[2].x);
    assert!(
        points
            .iter()
            .all(|point| point.x >= 0.0 && point.x <= 300.0)
    );
    assert!(
        points
            .iter()
            .all(|point| point.y >= 0.0 && point.y <= FLOW_CHART_HEIGHT)
    );
}

#[test]
fn cumulative_line_uses_bar_zero_baseline() {
    let values = [-100.0, 0.0, 100.0];
    let scale = flow_chart_scale(&values, FLOW_CHART_HEIGHT);
    let points = cumulative_line_points(&values, 300.0, FLOW_CHART_HEIGHT, scale);
    let zero_y = scale.zero_y;

    assert!(points[0].y > zero_y);
    assert_eq!(points[1].y, zero_y);
    assert!(points[2].y < zero_y);
}

#[test]
fn cumulative_line_points_skip_nonfinite_values() {
    let values = [100.0, f64::NAN, 50.0, f64::INFINITY];
    let scale = flow_chart_scale(&values, FLOW_CHART_HEIGHT);
    let points = cumulative_line_points(&values, 300.0, FLOW_CHART_HEIGHT, scale);

    assert_eq!(points.len(), 2);
    assert!(points.iter().all(|point| point.x.is_finite()));
    assert!(points.iter().all(|point| point.y.is_finite()));
}

#[test]
fn positive_only_bars_use_most_of_chart_height() {
    let values = [100.0, 50.0, 25.0];
    let scale = flow_chart_scale(&values, FLOW_CHART_HEIGHT);
    let (_top_spacer, positive_height, negative_height, bottom_spacer) =
        flow_bar_layout(100.0, scale);

    assert!(positive_height > FLOW_CHART_HEIGHT * 0.75);
    assert_eq!(negative_height, 0.0);
    assert!((bottom_spacer - scale.bottom_padding).abs() < 0.1);
}

#[test]
fn hype_etf_formatters_share_not_available_fallback() {
    let denomination = DisplayDenominationContext::default();

    assert_eq!(
        formatting::format_usd_value(Some(125.0), 2, &denomination),
        "$125.00"
    );
    assert_eq!(
        formatting::format_usd_value(Some(f64::NAN), 2, &denomination),
        "n/a"
    );
    assert_eq!(formatting::format_amount(None), "n/a");
    assert_eq!(formatting::format_hype(Some(f64::INFINITY)), "n/a");
    assert_eq!(formatting::format_pct(None), "n/a");
    assert_eq!(
        formatting::format_signed_usd_amount(0.004, &denomination),
        "$0.00"
    );
}

#[test]
fn hype_etf_formatters_preserve_display_rules() {
    assert_eq!(formatting::format_amount(Some(1_500_000.0)), "1.5M");
    assert_eq!(formatting::format_hype(Some(1_234.0)), "1,234 HYPE");
    assert_eq!(formatting::format_pct(Some(0.5)), "+0.50%");
    assert_eq!(formatting::format_pct(Some(-1.234)), "-1.23%");
    assert_eq!(formatting::short_flow_date("2026-05-20"), "05/20");
}
