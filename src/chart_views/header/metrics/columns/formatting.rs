use crate::denomination::format_compact_usd;
use crate::helpers::{format_price, invalid_data_placeholder, parse_finite_number};

// ---------------------------------------------------------------------------
// Metric Formatting
// ---------------------------------------------------------------------------

pub(super) fn open_interest_label(as_notional: bool) -> String {
    if as_notional {
        "Open Interest $".to_string()
    } else {
        "Open Interest".to_string()
    }
}

pub(super) fn format_open_interest(oi: Option<f64>, price: f64, as_notional: bool) -> String {
    let Some(oi) = oi else {
        return invalid_data_placeholder();
    };
    if as_notional {
        return format_open_interest_notional(oi, price);
    }
    if oi >= 1_000_000.0 {
        format!("{:.1}M", oi / 1_000_000.0)
    } else if oi >= 1_000.0 {
        format!("{:.0}K", oi / 1_000.0)
    } else {
        format!("{oi:.0}")
    }
}

pub(super) fn format_open_interest_notional(oi: f64, price: f64) -> String {
    if !oi.is_finite() || !price.is_finite() || oi < 0.0 || price <= 0.0 {
        return invalid_data_placeholder();
    }
    format_compact_usd(oi * price)
}

pub(super) fn format_volume(vlm: Option<f64>) -> String {
    let Some(vlm) = vlm else {
        return invalid_data_placeholder();
    };
    if !vlm.is_finite() || vlm < 0.0 {
        return invalid_data_placeholder();
    }
    format_compact_usd(vlm)
}

pub(super) fn format_metric_price(value: Option<f64>) -> String {
    value
        .map(format_price)
        .unwrap_or_else(invalid_data_placeholder)
}

pub(super) fn format_funding_pct(funding: Option<f64>) -> String {
    funding
        .map(|funding| format!("{:.4}%", funding * 100.0))
        .unwrap_or_else(invalid_data_placeholder)
}

pub(super) fn parse_ctx_f64(value: Option<&str>) -> Option<f64> {
    value.and_then(parse_finite_number)
}
