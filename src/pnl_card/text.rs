use crate::denomination::DisplayDenominationContext;
use crate::helpers::{format_signed_percent_value, normalize_two_decimal_display_value};
use std::fmt;

use super::metrics::PnlCardMetrics;
use super::model::{PnlCardDisplayMode, PnlCardPercentMode, PnlCardWindowState};

#[path = "text/privacy.rs"]
mod privacy;

#[cfg(test)]
pub(super) use privacy::obscure_price_digits;
pub(super) use privacy::privacy_price_display;

// ---------------------------------------------------------------------------
// Display Text
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub(super) struct PnlCardRenderText {
    pub(super) ticker: String,
    pub(super) leverage_display: String,
    pub(super) primary_value: String,
    pub(super) percent_mode_label: &'static str,
    pub(super) secondary_value: Option<String>,
    pub(super) entry_display: String,
    pub(super) exit_display: String,
    pub(super) context: String,
}

impl fmt::Debug for PnlCardRenderText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PnlCardRenderText")
            .field("ticker", &format_args!("<redacted>"))
            .field("leverage_display", &format_args!("<redacted>"))
            .field("primary_value", &format_args!("<redacted>"))
            .field("percent_mode_label", &self.percent_mode_label)
            .field("secondary_value_present", &self.secondary_value.is_some())
            .field("entry_display", &format_args!("<redacted>"))
            .field("exit_display", &format_args!("<redacted>"))
            .field("context", &format_args!("<redacted>"))
            .finish()
    }
}

impl PnlCardPercentMode {
    fn select(self, asset_move_pct: Option<f64>, leveraged_pct: Option<f64>) -> Option<f64> {
        match self {
            Self::AssetMove => asset_move_pct,
            Self::Leveraged => leveraged_pct,
        }
    }
}

pub(super) fn pnl_card_render_text(
    state: &PnlCardWindowState,
    metrics: &PnlCardMetrics,
    denomination: &DisplayDenominationContext,
) -> PnlCardRenderText {
    let percent = state
        .percent_mode
        .select(metrics.asset_move_pct, metrics.leveraged_pct);

    PnlCardRenderText {
        ticker: metrics.ticker.clone(),
        leverage_display: metrics.leverage_display.clone(),
        primary_value: pnl_card_primary_value(
            state.display_mode,
            percent,
            metrics.upnl,
            denomination,
        ),
        percent_mode_label: state.percent_mode.label(),
        secondary_value: pnl_card_secondary_value(state.display_mode, metrics.upnl, denomination),
        entry_display: privacy_price_display(&metrics.entry_display, state.obscure_prices),
        exit_display: privacy_price_display(&metrics.exit_display, state.obscure_prices),
        context: pnl_card_context_display(state, metrics),
    }
}

pub(super) fn pnl_card_context_display(
    state: &PnlCardWindowState,
    metrics: &PnlCardMetrics,
) -> String {
    if state.show_position_size {
        metrics.context.clone()
    } else {
        metrics
            .private_context
            .clone()
            .unwrap_or_else(|| metrics.context.clone())
    }
}

fn pnl_card_primary_value(
    display_mode: PnlCardDisplayMode,
    percent: Option<f64>,
    upnl: f64,
    denomination: &DisplayDenominationContext,
) -> String {
    match display_mode {
        PnlCardDisplayMode::PercentOnly | PnlCardDisplayMode::Both => percent
            .map(format_signed_percent_value)
            .unwrap_or_else(|| "--%".to_string()),
        PnlCardDisplayMode::UsdOnly => format_signed_usd(upnl, denomination),
    }
}

fn pnl_card_secondary_value(
    display_mode: PnlCardDisplayMode,
    upnl: f64,
    denomination: &DisplayDenominationContext,
) -> Option<String> {
    match display_mode {
        PnlCardDisplayMode::Both => Some(format_signed_usd(upnl, denomination)),
        PnlCardDisplayMode::PercentOnly | PnlCardDisplayMode::UsdOnly => None,
    }
}

fn format_signed_usd(value: f64, denomination: &DisplayDenominationContext) -> String {
    denomination.format_signed_value(normalize_two_decimal_display_value(value), 2)
}
