use super::super::super::metrics::{PositioningFlowRow, format_signed_size};
use super::PositioningFlowChartRow;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::trim_decimal_zeros;

pub(super) fn build_chart_row(
    row: &PositioningFlowRow,
    usd_scaled: bool,
    denomination: &DisplayDenominationContext,
) -> PositioningFlowChartRow {
    let more_long = row.delta_size >= 0.0;
    let value_text = if usd_scaled {
        match row.delta_usd {
            Some(usd) => signed_compact_usd(usd, denomination),
            None => signed_compact_size(row.delta_size),
        }
    } else {
        signed_compact_size(row.delta_size)
    };

    let mut tooltip: Vec<(String, String)> = Vec::with_capacity(5);
    tooltip.push(("Kind".to_string(), row.kind.label().to_string()));
    tooltip.push((
        "Prev".to_string(),
        row.previous_size
            .map(|value| format_signed_size(value, true))
            .unwrap_or_else(|| "-".to_string()),
    ));
    tooltip.push((
        "Now".to_string(),
        format_signed_size(row.current_size, true),
    ));
    tooltip.push((
        "Change".to_string(),
        format_signed_size(row.delta_size, true),
    ));
    if usd_scaled {
        if let Some(usd) = row.delta_usd {
            tooltip.push((
                "Change $".to_string(),
                denomination.format_signed_value(usd, 0),
            ));
        }
        if let Some(usd) = row.current_usd {
            tooltip.push((
                "Now $".to_string(),
                denomination.format_signed_value(usd, 0),
            ));
        }
    }

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

fn signed_compact_size(value: f64) -> String {
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
    format!("{sign}{}", compact_number(value.abs()))
}

pub(super) fn compact_size(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    compact_number(value.abs())
}

fn signed_compact_usd(value: f64, denomination: &DisplayDenominationContext) -> String {
    let Some(display) = denomination.convert_usd_value(value) else {
        return "-".to_string();
    };
    let sign = if display > 0.0 {
        "+"
    } else if display < 0.0 {
        "-"
    } else {
        ""
    };
    denomination.format_active_amount(sign, compact_number(display.abs()))
}

pub(super) fn compact_usd(value: f64, denomination: &DisplayDenominationContext) -> String {
    let Some(display) = denomination.convert_usd_value(value) else {
        return "-".to_string();
    };
    denomination.format_active_amount("", compact_number(display.abs()))
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
