use crate::api::OutcomeVolume24h;
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

pub(super) fn asset_volume_label(as_notional: bool) -> String {
    if as_notional {
        "24h Vol $".to_string()
    } else {
        "24h Vol".to_string()
    }
}

pub(super) fn format_asset_volume(
    base_volume: Option<f64>,
    notional_volume: Option<f64>,
    as_notional: bool,
    symbol: &str,
) -> String {
    if as_notional {
        return format_volume(notional_volume);
    }

    let Some(base_volume) = base_volume else {
        return invalid_data_placeholder();
    };
    if !base_volume.is_finite() || base_volume < 0.0 {
        return invalid_data_placeholder();
    }

    format!("{} {}", format_compact_amount(base_volume), symbol)
}

pub(super) fn outcome_volume_label(as_notional: bool) -> String {
    if as_notional {
        "24h Vol $".to_string()
    } else {
        "24h Vol".to_string()
    }
}

pub(super) fn format_outcome_volume(volume: OutcomeVolume24h, as_notional: bool) -> String {
    if as_notional {
        format_volume(Some(volume.notional))
    } else {
        format_outcome_contract_volume(volume.contract)
    }
}

pub(super) fn format_outcome_asset_volume(
    base_volume: Option<f64>,
    notional_volume: Option<f64>,
    fallback: Option<OutcomeVolume24h>,
    as_notional: bool,
) -> String {
    if as_notional {
        format_volume(notional_volume.or_else(|| fallback.map(|volume| volume.notional)))
    } else {
        let contract_volume = base_volume.or_else(|| fallback.map(|volume| volume.contract));
        contract_volume
            .map(format_outcome_contract_volume)
            .unwrap_or_else(invalid_data_placeholder)
    }
}

fn format_outcome_contract_volume(value: f64) -> String {
    if !value.is_finite() || value < 0.0 {
        return invalid_data_placeholder();
    }

    if value >= 1_000_000_000.0 {
        format!("{:.1}B contracts", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("{:.1}M contracts", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}K contracts", value / 1_000.0)
    } else if value >= 1.0 {
        format!("{value:.0} contracts")
    } else {
        format!("{value:.2} contracts")
    }
}

fn format_compact_amount(value: f64) -> String {
    if value >= 1_000_000_000.0 {
        format!("{:.1}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else if value >= 1.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.4}")
    }
}

/// Spot `dayBaseVlm` is denominated purely in the base token, but spot chart
/// instances carry the pair display name ("HYPE/USDC"); label base-unit
/// volume with the base ticker so it does not read as pair units.
pub(super) fn spot_base_ticker(symbol_display: &str) -> &str {
    symbol_display
        .split_once('/')
        .map(|(base, _quote)| base)
        .unwrap_or(symbol_display)
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
