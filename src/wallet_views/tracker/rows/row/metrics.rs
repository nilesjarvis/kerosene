use crate::denomination::DisplayDenominationContext;
use crate::wallet_state::WalletTrackerRow;

use iced::widget::{Text, text};
use iced::{Color, Theme};

pub(super) struct WalletRowMetrics {
    pub(super) equity: String,
    pub(super) withdrawable: String,
    pub(super) upnl: String,
    pub(super) margin: String,
    pub(super) risk: String,
    pub(super) data_color: Color,
    raw_upnl: Option<f64>,
    loaded: bool,
}

pub(super) fn wallet_row_metrics(
    row_data: &WalletTrackerRow,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> WalletRowMetrics {
    if let Some(snapshot) = row_data.snapshot.as_ref() {
        let exposure = snapshot
            .long_exposure
            .zip(snapshot.short_exposure)
            .map(|(long, short)| long + short);
        let order_count = if row_data.order_loading {
            "...o".to_string()
        } else if row_data.order_error.is_some() {
            "err".to_string()
        } else {
            row_data
                .open_order_count
                .map(|count| format!("{count}o"))
                .unwrap_or_else(|| "-o".to_string())
        };
        let trade_count = snapshot
            .open_trade_count
            .map(|count| format!("{count}p"))
            .unwrap_or_else(invalid_tracker_data);
        let has_invalid_data = snapshot.equity.is_none()
            || snapshot.withdrawable.is_none()
            || snapshot.unrealized_pnl.is_none()
            || snapshot.margin_used_pct.is_none()
            || snapshot.open_trade_count.is_none()
            || snapshot.long_exposure.is_none()
            || snapshot.short_exposure.is_none();

        return WalletRowMetrics {
            equity: money_display(snapshot.equity, denomination),
            withdrawable: money_display(snapshot.withdrawable, denomination),
            upnl: signed_money_display(snapshot.unrealized_pnl, denomination),
            margin: snapshot
                .margin_used_pct
                .map(|margin| format!("{margin:.1}%"))
                .unwrap_or_else(invalid_tracker_data),
            risk: format!(
                "{trade_count} / {order_count} | {}",
                exposure
                    .map(|exposure| denomination.format_value(exposure, 0))
                    .unwrap_or_else(invalid_tracker_data)
            ),
            data_color: if has_invalid_data {
                theme.palette().warning
            } else {
                theme.palette().text
            },
            raw_upnl: snapshot.unrealized_pnl,
            loaded: true,
        };
    }

    WalletRowMetrics {
        equity: "-".to_string(),
        withdrawable: "-".to_string(),
        upnl: "-".to_string(),
        margin: "-".to_string(),
        risk: "-".to_string(),
        data_color: theme.extended_palette().background.weak.text,
        raw_upnl: None,
        loaded: false,
    }
}

pub(super) fn wallet_upnl_color(metrics: &WalletRowMetrics, theme: &Theme) -> Color {
    match metrics.raw_upnl {
        Some(upnl) if upnl > 0.0 => theme.palette().success,
        Some(upnl) if upnl < 0.0 => theme.palette().danger,
        Some(_) => theme.extended_palette().background.weak.text,
        None if metrics.loaded => theme.palette().warning,
        None => theme.extended_palette().background.weak.text,
    }
}

pub(super) fn money_text(value: String, color: Color) -> Text<'static> {
    text(value)
        .size(11)
        .font(crate::app_fonts::monospace_font())
        .color(color)
}

fn money_display(value: Option<f64>, denomination: &DisplayDenominationContext) -> String {
    value
        .map(|value| denomination.format_value(value, 2))
        .unwrap_or_else(invalid_tracker_data)
}

fn signed_money_display(value: Option<f64>, denomination: &DisplayDenominationContext) -> String {
    value
        .map(|value| denomination.format_signed_value(value, 2))
        .unwrap_or_else(invalid_tracker_data)
}

fn invalid_tracker_data() -> String {
    "Invalid data".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::WalletTrackerSnapshot;

    #[test]
    fn wallet_row_metrics_show_placeholders_without_snapshot() {
        let row = WalletTrackerRow::default();
        let denomination = DisplayDenominationContext::default();
        let metrics = wallet_row_metrics(&row, &denomination, &Theme::Dark);

        assert_eq!(metrics.equity, "-");
        assert_eq!(metrics.withdrawable, "-");
        assert_eq!(metrics.upnl, "-");
        assert_eq!(metrics.margin, "-");
        assert_eq!(metrics.risk, "-");
    }

    #[test]
    fn wallet_row_metrics_mark_invalid_snapshot_values() {
        let row = WalletTrackerRow {
            snapshot: Some(WalletTrackerSnapshot {
                equity: None,
                withdrawable: Some(10.0),
                unrealized_pnl: None,
                margin_used_pct: None,
                open_trade_count: None,
                open_order_count: 0,
                long_exposure: None,
                short_exposure: Some(0.0),
            }),
            ..WalletTrackerRow::default()
        };

        let denomination = DisplayDenominationContext::default();
        let metrics = wallet_row_metrics(&row, &denomination, &Theme::Dark);

        assert_eq!(metrics.equity, "Invalid data");
        assert_eq!(metrics.withdrawable, "$10.00");
        assert_eq!(metrics.upnl, "Invalid data");
        assert_eq!(metrics.margin, "Invalid data");
        assert_eq!(metrics.risk, "Invalid data / -o | Invalid data");
    }
}
