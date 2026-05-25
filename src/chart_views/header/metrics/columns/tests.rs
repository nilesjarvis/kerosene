use super::{
    ChartHeaderMetricVisibility, format_funding_pct, format_open_interest,
    format_open_interest_notional, format_volume, open_interest_label, parse_ctx_f64,
};

#[test]
fn context_number_parser_rejects_missing_malformed_or_nonfinite_values() {
    assert_eq!(parse_ctx_f64(Some("12.5")), Some(12.5));
    assert_eq!(parse_ctx_f64(Some(" 12.5 ")), Some(12.5));
    assert_eq!(parse_ctx_f64(None), None);
    assert_eq!(parse_ctx_f64(Some("bad")), None);
    assert_eq!(parse_ctx_f64(Some("NaN")), None);
    assert_eq!(parse_ctx_f64(Some("inf")), None);
}

#[test]
fn header_metric_formatters_mark_invalid_values() {
    assert_eq!(format_volume(None), "Invalid data");
    assert_eq!(format_open_interest(None, 100.0, false), "Invalid data");
    assert_eq!(format_funding_pct(None), "Invalid data");
    assert_eq!(format_volume(Some(1_500.0)), "$1.5K");
    assert_eq!(
        format_open_interest(Some(1_500_000.0), 100.0, false),
        "1.5M"
    );
    assert_eq!(format_funding_pct(Some(0.0001)), "0.0100%");
}

#[test]
fn open_interest_notional_formats_from_chart_price() {
    assert_eq!(format_open_interest(Some(1_500.0), 2_000.0, true), "$3.00M");
    assert_eq!(
        format_open_interest_notional(2_000_000.0, 2_000.0),
        "$4.00B"
    );
    assert_eq!(
        format_open_interest(Some(1_500.0), 0.0, true),
        "Invalid data"
    );
    assert_eq!(open_interest_label(false), "Open Interest");
    assert_eq!(open_interest_label(true), "Open Interest $");
}

#[test]
fn metric_visibility_collapses_in_priority_order() {
    assert_eq!(
        ChartHeaderMetricVisibility::for_width(760.0),
        ChartHeaderMetricVisibility {
            show_24h_change: true,
            show_mark_oracle: true,
            show_open_interest: true,
            show_funding: true,
        }
    );
    assert_eq!(
        ChartHeaderMetricVisibility::for_width(680.0),
        ChartHeaderMetricVisibility {
            show_24h_change: true,
            show_mark_oracle: false,
            show_open_interest: true,
            show_funding: true,
        }
    );
    assert_eq!(
        ChartHeaderMetricVisibility::for_width(520.0),
        ChartHeaderMetricVisibility {
            show_24h_change: true,
            show_mark_oracle: false,
            show_open_interest: false,
            show_funding: true,
        }
    );
    assert_eq!(
        ChartHeaderMetricVisibility::for_width(420.0),
        ChartHeaderMetricVisibility {
            show_24h_change: true,
            show_mark_oracle: false,
            show_open_interest: false,
            show_funding: false,
        }
    );
    assert_eq!(
        ChartHeaderMetricVisibility::for_width(320.0),
        ChartHeaderMetricVisibility {
            show_24h_change: false,
            show_mark_oracle: false,
            show_open_interest: false,
            show_funding: false,
        }
    );
}
