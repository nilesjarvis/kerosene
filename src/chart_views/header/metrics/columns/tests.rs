use super::{
    ChartHeaderMetricVisibility, asset_volume_label, format_asset_volume, format_funding_pct,
    format_open_interest, format_open_interest_notional, format_outcome_asset_volume,
    format_outcome_volume, format_volume, open_interest_label, outcome_volume_label, parse_ctx_f64,
};
use crate::api::OutcomeVolume24h;

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
fn outcome_volume_formats_contracts_and_notional() {
    let volume = OutcomeVolume24h {
        contract: 18_055.0,
        notional: 4_513.75,
    };

    assert_eq!(format_outcome_volume(volume, false), "18.1K contracts");
    assert_eq!(format_outcome_volume(volume, true), "$4.5K");
    assert_eq!(outcome_volume_label(false), "24h Vol");
    assert_eq!(outcome_volume_label(true), "24h Vol $");
}

#[test]
fn outcome_asset_volume_prefers_live_context_and_falls_back_to_candle_volume() {
    let fallback = OutcomeVolume24h {
        contract: 18_055.0,
        notional: 4_513.75,
    };

    assert_eq!(
        format_outcome_asset_volume(Some(20_000.0), Some(5_000.0), Some(fallback), false),
        "20.0K contracts"
    );
    assert_eq!(
        format_outcome_asset_volume(Some(20_000.0), Some(5_000.0), Some(fallback), true),
        "$5.0K"
    );
    assert_eq!(
        format_outcome_asset_volume(None, Some(5_000.0), Some(fallback), false),
        "18.1K contracts"
    );
    assert_eq!(
        format_outcome_asset_volume(Some(20_000.0), None, Some(fallback), true),
        "$4.5K"
    );
    assert_eq!(
        format_outcome_asset_volume(None, None, None, false),
        "Invalid data"
    );
}

#[test]
fn asset_volume_formats_coin_and_notional_modes() {
    assert_eq!(
        format_asset_volume(Some(1_500.0), Some(3_750_000.0), false, "HYPE"),
        "1.5K HYPE"
    );
    assert_eq!(
        format_asset_volume(Some(1_500.0), Some(3_750_000.0), true, "HYPE"),
        "$3.75M"
    );
    assert_eq!(
        format_asset_volume(None, Some(3_750_000.0), false, "HYPE"),
        "Invalid data"
    );
    assert_eq!(
        format_asset_volume(Some(1_500.0), None, true, "HYPE"),
        "Invalid data"
    );
    assert_eq!(asset_volume_label(false), "24h Vol");
    assert_eq!(asset_volume_label(true), "24h Vol $");
}

#[test]
fn metric_visibility_collapses_in_priority_order() {
    assert_eq!(
        ChartHeaderMetricVisibility::for_width(760.0),
        ChartHeaderMetricVisibility {
            show_24h_change: true,
            show_24h_volume: true,
            show_mark_oracle: true,
            show_open_interest: true,
            show_funding: true,
        }
    );
    assert_eq!(
        ChartHeaderMetricVisibility::for_width(680.0),
        ChartHeaderMetricVisibility {
            show_24h_change: true,
            show_24h_volume: true,
            show_mark_oracle: false,
            show_open_interest: true,
            show_funding: true,
        }
    );
    assert_eq!(
        ChartHeaderMetricVisibility::for_width(520.0),
        ChartHeaderMetricVisibility {
            show_24h_change: true,
            show_24h_volume: true,
            show_mark_oracle: false,
            show_open_interest: false,
            show_funding: true,
        }
    );
    assert_eq!(
        ChartHeaderMetricVisibility::for_width(420.0),
        ChartHeaderMetricVisibility {
            show_24h_change: true,
            show_24h_volume: true,
            show_mark_oracle: false,
            show_open_interest: false,
            show_funding: false,
        }
    );
    assert_eq!(
        ChartHeaderMetricVisibility::for_width(320.0),
        ChartHeaderMetricVisibility {
            show_24h_change: false,
            show_24h_volume: false,
            show_mark_oracle: false,
            show_open_interest: false,
            show_funding: false,
        }
    );
}
