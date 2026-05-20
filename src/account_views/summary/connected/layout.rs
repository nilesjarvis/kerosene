use super::super::CONNECTED_SUMMARY_ACTION_BREAKPOINT;
use super::metrics::ConnectedSummaryValues;
use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::helpers::vertical_spacer;
use crate::message::Message;
use iced::widget::{Row, Space, column, container, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn connected_summary_base_row<'a>(&'a self) -> Row<'a, Message> {
        row![self.summary_account_picker(), vertical_spacer(),]
            .spacing(16)
            .align_y(iced::Alignment::Center)
            .width(Fill)
    }

    pub(super) fn view_connected_summary_layout<'a>(
        &'a self,
        data: &AccountData,
        theme: &Theme,
        available_width: f32,
    ) -> Element<'a, Message> {
        let summary_values = self.connected_summary_values(data);
        let metrics = self.connected_summary_metrics_row(data, &summary_values, theme);

        if available_width < CONNECTED_SUMMARY_ACTION_BREAKPOINT {
            column![
                metrics,
                row![
                    Space::new().width(Fill),
                    self.connected_summary_actions_row()
                ]
                .width(Fill)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(6)
            .width(Fill)
            .into()
        } else {
            self.push_connected_summary_actions(metrics).into()
        }
    }

    fn connected_summary_metrics_row<'a>(
        &'a self,
        data: &AccountData,
        summary_values: &ConnectedSummaryValues,
        theme: &Theme,
    ) -> Row<'a, Message> {
        let items = self.connected_summary_base_row();
        self.push_connected_summary_metrics(items, data, summary_values, theme)
            .width(Fill)
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
                .push(summary_metric_colored(
                    "Mode",
                    "Portfolio",
                    theme.palette().primary,
                    theme,
                ))
                .push(vertical_spacer())
                .push(summary_metric(
                    "Total Value",
                    self.mask_connected_summary_usd(&summary_values.total_value),
                    theme,
                ))
                .push(vertical_spacer())
                .push(summary_metric_colored(
                    "Available",
                    self.mask_connected_summary_usd(&summary_values.available_value),
                    avail_color,
                    theme,
                ))
                .push(vertical_spacer())
                .push(summary_metric(
                    "Notional Pos",
                    self.mask_connected_summary_usd(&summary_values.live_notional),
                    theme,
                ))
                .push(vertical_spacer())
                .push(summary_metric(
                    "Eff Lev",
                    &summary_values.effective_leverage_value,
                    theme,
                ))
                .push(vertical_spacer())
                .push(summary_metric(
                    "Margin Used",
                    self.mask_connected_summary_usd(&summary_values.margin_used_value),
                    theme,
                ))
                .push(vertical_spacer())
                .push(summary_metric_colored(
                    "Margin Ratio",
                    &summary_values.portfolio_margin_ratio_value,
                    ratio_color,
                    theme,
                ))
        } else {
            items
                .push(summary_metric(
                    "Total Value",
                    self.mask_connected_summary_usd(&summary_values.total_value),
                    theme,
                ))
                .push(vertical_spacer())
                .push(summary_metric_colored(
                    "Available",
                    self.mask_connected_summary_usd(&summary_values.available_value),
                    avail_color,
                    theme,
                ))
                .push(vertical_spacer())
                .push(summary_metric(
                    "Notional Pos",
                    self.mask_connected_summary_usd(&summary_values.live_notional),
                    theme,
                ))
                .push(vertical_spacer())
                .push(summary_metric(
                    "Eff Lev",
                    &summary_values.effective_leverage_value,
                    theme,
                ))
                .push(vertical_spacer())
                .push(summary_metric(
                    "Margin Used",
                    self.mask_connected_summary_usd(&summary_values.margin_used_value),
                    theme,
                ))
        }
    }

    pub(super) fn push_connected_summary_actions<'a>(
        &'a self,
        items: Row<'a, Message>,
    ) -> Row<'a, Message> {
        items
            .push(Space::new().width(Fill))
            .push(self.connected_summary_actions_row())
            .width(Fill)
    }

    fn connected_summary_actions_row<'a>(&'a self) -> Row<'a, Message> {
        row![
            self.summary_market_universe_picker(),
            self.summary_display_denomination_picker(),
            self.summary_hide_pnl_button(),
            self.summary_sound_button(),
            self.summary_notifications_button(),
            self.summary_layouts_button(),
            self.summary_widgets_button(),
            self.summary_settings_button(),
            self.summary_disconnect_button(),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
    }

    fn mask_connected_summary_usd(&self, value: &str) -> String {
        if self.hide_pnl {
            self.display_pnl_mask()
        } else {
            self.format_display_usd_str(value)
        }
    }
}

fn summary_metric(
    label: &'static str,
    value: impl ToString,
    theme: &Theme,
) -> Element<'static, Message> {
    summary_metric_with_color(label, value, None, theme)
}

fn summary_metric_colored(
    label: &'static str,
    value: impl ToString,
    value_color: Color,
    theme: &Theme,
) -> Element<'static, Message> {
    summary_metric_with_color(label, value, Some(value_color), theme)
}

fn summary_metric_with_color(
    label: &'static str,
    value: impl ToString,
    value_color: Option<Color>,
    theme: &Theme,
) -> Element<'static, Message> {
    let value = text(value.to_string()).size(13).font(iced::Font::MONOSPACE);
    let value = if let Some(value_color) = value_color {
        value.color(value_color)
    } else {
        value
    };

    container(
        column![
            text(label)
                .size(10)
                .color(theme.extended_palette().background.weak.text),
            value,
        ]
        .spacing(1)
        .align_x(iced::Alignment::Start),
    )
    .into()
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
