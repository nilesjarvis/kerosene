use super::metrics::ConnectedSummaryValues;
use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::helpers::{format_usd, label_value, label_value_colored, vertical_spacer};
use crate::message::Message;
use iced::widget::{Row, Space, button, row, text};
use iced::{Fill, Theme};

impl TradingTerminal {
    pub(super) fn connected_account_label(&self, addr: &str) -> String {
        let account_display = self.wallet_display(addr);
        if account_display.has_label {
            format!(
                "{} ({})",
                account_display.primary, account_display.secondary
            )
        } else {
            account_display.primary
        }
    }

    pub(super) fn connected_summary_base_row<'a>(
        &'a self,
        addr: &str,
        account_label: &str,
        theme: &Theme,
    ) -> Row<'a, Message> {
        let copy_btn = button(
            text("Copy")
                .size(10)
                .color(theme.extended_palette().background.weak.text),
        )
        .on_press(Message::CopyToClipboard(addr.to_string()))
        .padding(0)
        .style(button::text);

        row![
            self.summary_account_picker(),
            vertical_spacer(),
            row![label_value("Account", account_label), copy_btn]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            vertical_spacer(),
        ]
        .spacing(16)
        .align_y(iced::Alignment::Center)
    }

    pub(super) fn push_connected_summary_metrics<'a>(
        &self,
        items: Row<'a, Message>,
        data: &AccountData,
        summary_values: &ConnectedSummaryValues,
        theme: &Theme,
    ) -> Row<'a, Message> {
        let avail_color = available_color(summary_values, theme);

        if data.is_portfolio_margin() {
            let ratio_color = portfolio_margin_ratio_color(summary_values, theme);

            items
                .push(label_value_colored(
                    "Mode",
                    "Portfolio",
                    theme.palette().primary,
                ))
                .push(vertical_spacer())
                .push(label_value(
                    "Total Value",
                    self.mask_connected_summary_usd(&summary_values.total_value),
                ))
                .push(vertical_spacer())
                .push(label_value_colored(
                    "Available",
                    self.mask_connected_summary_usd(&summary_values.available_value),
                    avail_color,
                ))
                .push(vertical_spacer())
                .push(label_value(
                    "Notional Pos",
                    self.mask_connected_summary_usd(&summary_values.live_notional),
                ))
                .push(vertical_spacer())
                .push(label_value(
                    "Eff Lev",
                    &summary_values.effective_leverage_value,
                ))
                .push(vertical_spacer())
                .push(label_value(
                    "Margin Used",
                    self.mask_connected_summary_usd(&summary_values.margin_used_value),
                ))
                .push(vertical_spacer())
                .push(label_value_colored(
                    "Margin Ratio",
                    &summary_values.portfolio_margin_ratio_value,
                    ratio_color,
                ))
        } else {
            items
                .push(label_value(
                    "Total Value",
                    self.mask_connected_summary_usd(&summary_values.total_value),
                ))
                .push(vertical_spacer())
                .push(label_value_colored(
                    "Available",
                    self.mask_connected_summary_usd(&summary_values.available_value),
                    avail_color,
                ))
                .push(vertical_spacer())
                .push(label_value(
                    "Notional Pos",
                    self.mask_connected_summary_usd(&summary_values.live_notional),
                ))
                .push(vertical_spacer())
                .push(label_value(
                    "Eff Lev",
                    &summary_values.effective_leverage_value,
                ))
                .push(vertical_spacer())
                .push(label_value(
                    "Margin Used",
                    self.mask_connected_summary_usd(&summary_values.margin_used_value),
                ))
        }
    }

    pub(super) fn push_connected_summary_actions<'a>(
        &'a self,
        items: Row<'a, Message>,
    ) -> Row<'a, Message> {
        items
            .push(Space::new().width(Fill))
            .push(self.summary_hide_pnl_button())
            .push(self.summary_sound_button())
            .push(self.summary_notifications_button())
            .push(self.summary_layouts_button())
            .push(self.summary_widgets_button())
            .push(self.summary_settings_button())
            .push(self.summary_disconnect_button())
    }

    fn mask_connected_summary_usd(&self, value: &str) -> String {
        if self.hide_pnl {
            "$***".to_string()
        } else {
            format_usd(value)
        }
    }
}

fn portfolio_margin_ratio_color(
    summary_values: &ConnectedSummaryValues,
    theme: &Theme,
) -> iced::Color {
    let Some(margin_ratio) = summary_values.portfolio_margin_ratio else {
        return theme.palette().warning;
    };
    if margin_ratio < 0.5 {
        theme.palette().success
    } else if margin_ratio < 0.8 {
        theme.palette().primary
    } else {
        theme.palette().danger
    }
}

fn available_color(summary_values: &ConnectedSummaryValues, theme: &Theme) -> iced::Color {
    let (Some(margin_used), Some(available)) =
        (summary_values.margin_used, summary_values.available)
    else {
        return theme.palette().warning;
    };
    if margin_used < 1e-6 || available > margin_used * 2.0 {
        theme.palette().success
    } else if available > margin_used * 0.5 {
        theme.palette().primary
    } else {
        theme.palette().danger
    }
}
