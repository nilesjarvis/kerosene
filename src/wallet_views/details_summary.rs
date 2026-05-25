mod cards;
mod metrics;

use self::cards::{summary_metric_card, summary_pm_status_line};

use crate::account::WalletDetailsData;
use crate::app_state::TradingTerminal;
use crate::helpers::optional_value_color;
use crate::message::Message;
use crate::wallet_views::numbers::{
    format_wallet_display_signed_usd, format_wallet_display_usd, invalid_wallet_data,
};
use crate::wallet_views::style::wallet_signed_value_color;

use iced::widget::{column, row};
use iced::{Color, Element, Theme};

// ---------------------------------------------------------------------------
// Wallet Details Summary
// ---------------------------------------------------------------------------

struct WalletDetailsSummaryMetrics {
    account_value: Option<f64>,
    withdrawable: Option<f64>,
    margin_pct: Option<f64>,
    notional: Option<f64>,
    long_exposure: Option<f64>,
    short_exposure: Option<f64>,
    unrealized_pnl: Option<f64>,
    active_position_count: usize,
    open_order_count: usize,
    non_zero_spot_count: usize,
    pm_ratio: Option<f64>,
    pm_available: String,
    portfolio_margin_enabled: bool,
}

impl TradingTerminal {
    pub(super) fn view_wallet_details_summary<'a>(
        &'a self,
        data: &'a WalletDetailsData,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let metrics = self.wallet_details_summary_metrics(data);
        let denomination = self.display_denomination_context();

        column![
            row![
                summary_metric_card(
                    "Equity",
                    format_wallet_display_usd(&denomination, metrics.account_value, 2),
                    optional_value_color(
                        metrics.account_value,
                        theme.palette().text,
                        theme.palette().warning
                    ),
                    theme
                ),
                summary_metric_card(
                    "Available",
                    format_wallet_display_usd(&denomination, metrics.withdrawable, 2),
                    optional_value_color(
                        metrics.withdrawable,
                        theme.palette().text,
                        theme.palette().warning
                    ),
                    theme
                ),
                summary_metric_card(
                    "uPnL",
                    format_wallet_display_signed_usd(&denomination, metrics.unrealized_pnl),
                    wallet_signed_value_color(metrics.unrealized_pnl, theme),
                    theme
                ),
                summary_metric_card(
                    "Margin",
                    metrics
                        .margin_pct
                        .map(|margin_pct| format!("{margin_pct:.1}%"))
                        .unwrap_or_else(invalid_wallet_data),
                    wallet_margin_color(metrics.margin_pct, theme),
                    theme
                ),
            ]
            .spacing(8),
            row![
                summary_metric_card(
                    "Notional",
                    format_wallet_display_usd(&denomination, metrics.notional, 2),
                    optional_value_color(
                        metrics.notional,
                        theme.palette().text,
                        theme.palette().warning
                    ),
                    theme
                ),
                summary_metric_card(
                    "Long / Short",
                    wallet_long_short_text(
                        &denomination,
                        metrics.long_exposure,
                        metrics.short_exposure,
                    ),
                    wallet_long_short_color(metrics.long_exposure, metrics.short_exposure, theme),
                    theme
                ),
                summary_metric_card(
                    "Positions / Orders",
                    format!(
                        "{} / {}",
                        metrics.active_position_count, metrics.open_order_count
                    ),
                    theme.palette().text,
                    theme
                ),
                summary_metric_card(
                    "Spot / PM",
                    format!(
                        "{} / {}",
                        metrics.non_zero_spot_count,
                        if metrics.portfolio_margin_enabled {
                            "on"
                        } else {
                            "off"
                        }
                    ),
                    theme.palette().text,
                    theme
                ),
            ]
            .spacing(8),
            summary_pm_status_line(metrics.pm_ratio, metrics.pm_available, theme),
        ]
        .spacing(8)
        .into()
    }
}

fn wallet_margin_color(value: Option<f64>, theme: &Theme) -> Color {
    match value {
        Some(value) if value >= 80.0 => theme.palette().danger,
        Some(value) if value >= 50.0 => theme.palette().primary,
        Some(_) => theme.palette().text,
        None => theme.palette().warning,
    }
}

fn wallet_long_short_text(
    denomination: &crate::denomination::DisplayDenominationContext,
    long_exposure: Option<f64>,
    short_exposure: Option<f64>,
) -> String {
    match (long_exposure, short_exposure) {
        (Some(long_exposure), Some(short_exposure)) => format!(
            "{} / {}",
            format_wallet_display_usd(denomination, Some(long_exposure), 0),
            format_wallet_display_usd(denomination, Some(short_exposure), 0)
        ),
        _ => invalid_wallet_data(),
    }
}

fn wallet_long_short_color(
    long_exposure: Option<f64>,
    short_exposure: Option<f64>,
    theme: &Theme,
) -> Color {
    if long_exposure.is_some() && short_exposure.is_some() {
        theme.palette().text
    } else {
        theme.palette().warning
    }
}
