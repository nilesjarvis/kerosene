use super::super::super::metrics::PositioningFlowRow;
use super::PositioningFlowChartRow;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::trim_decimal_zeros;

pub(super) fn build_chart_row(row: &PositioningFlowRow) -> PositioningFlowChartRow {
    let more_long = row.delta_usd >= 0.0;
    let value_text = signed_compact_usd(row.delta_usd);
    let denomination = DisplayDenominationContext::usd();
    let format_exposure = |value: Option<f64>| {
        value
            .map(|usd| denomination.format_signed_value(usd, 2))
            .unwrap_or_else(|| "-".to_string())
    };

    let tooltip = vec![
        ("Kind".to_string(), row.kind.label().to_string()),
        ("Prev".to_string(), format_exposure(row.previous_usd)),
        ("Now".to_string(), format_exposure(row.current_usd)),
        ("Change".to_string(), format_exposure(Some(row.delta_usd))),
    ];

    PositioningFlowChartRow {
        address: row.address.clone(),
        label: row.address.clone(),
        hover_key: String::new(),
        value_text,
        magnitude: row.magnitude(),
        more_long,
        kind: row.kind,
        tooltip,
    }
}

pub(super) fn signed_compact_usd(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    let sign = if value > 0.0 {
        "+"
    } else if value < 0.0 {
        "-"
    } else {
        ""
    };
    format!("{sign}${}", compact_number(value.abs()))
}

pub(super) fn compact_usd(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    format!("${}", compact_number(value.abs()))
}

fn compact_number(value: f64) -> String {
    let value = value.abs();
    if value >= 1_000_000_000.0 {
        trim_decimal_zeros(format!("{:.1}", value / 1_000_000_000.0)) + "b"
    } else if value >= 1_000_000.0 {
        trim_decimal_zeros(format!("{:.1}", value / 1_000_000.0)) + "m"
    } else if value >= 1_000.0 {
        trim_decimal_zeros(format!("{:.1}", value / 1_000.0)) + "k"
    } else {
        trim_decimal_zeros(format!("{value:.2}"))
    }
}
