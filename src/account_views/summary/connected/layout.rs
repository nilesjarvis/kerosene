use super::super::CONNECTED_SUMMARY_ACTION_BREAKPOINT;
use super::metrics::ConnectedSummaryValues;
use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::helpers::vertical_spacer;
use crate::message::Message;
use iced::widget::{Row, Space, column, container, row, text};
use iced::{Color, Element, Fill, Theme};

const HIDE_DISPLAY_DENOMINATION_SELECTOR_WIDTH: f32 = CONNECTED_SUMMARY_ACTION_BREAKPOINT;
const HIDE_SOUND_SELECTOR_WIDTH: f32 = 1_020.0;
const HIDE_NOTIFICATION_SELECTOR_WIDTH: f32 = 940.0;
const HIDE_MARGIN_RATIO_WIDTH: f32 = 840.0;
const HIDE_MARGIN_USED_WIDTH: f32 = 720.0;

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
        let compaction = ConnectedSummaryCompaction::for_width(available_width);
        let summary_values = self.connected_summary_values(data);
        let metrics = self.connected_summary_metrics_row(data, &summary_values, theme, compaction);

        if available_width < CONNECTED_SUMMARY_ACTION_BREAKPOINT {
            column![
                metrics,
                row![
                    Space::new().width(Fill),
                    self.connected_summary_actions_row(compaction)
                ]
                .width(Fill)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(6)
            .width(Fill)
            .into()
        } else {
            self.push_connected_summary_actions(metrics, compaction)
                .into()
        }
    }

    fn connected_summary_metrics_row<'a>(
        &'a self,
        data: &AccountData,
        summary_values: &ConnectedSummaryValues,
        theme: &Theme,
        compaction: ConnectedSummaryCompaction,
    ) -> Row<'a, Message> {
        let items = self.connected_summary_base_row();
        self.push_connected_summary_metrics(items, data, summary_values, theme, compaction)
            .width(Fill)
    }

    fn push_connected_summary_metrics<'a>(
        &self,
        items: Row<'a, Message>,
        data: &AccountData,
        summary_values: &ConnectedSummaryValues,
        theme: &Theme,
        compaction: ConnectedSummaryCompaction,
    ) -> Row<'a, Message> {
        let avail_color = available_color(summary_values, theme);

        if data.is_portfolio_margin() {
            let ratio_color = portfolio_margin_ratio_color(summary_values, theme);

            let items = items
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
                ));

            let items = if compaction.hide_margin_used() {
                items
            } else {
                items.push(vertical_spacer()).push(summary_metric(
                    "Margin Used",
                    self.mask_connected_summary_usd(&summary_values.margin_used_value),
                    theme,
                ))
            };

            if compaction.hide_margin_ratio() {
                items
            } else {
                items.push(vertical_spacer()).push(summary_metric_colored(
                    "Margin Ratio",
                    &summary_values.portfolio_margin_ratio_value,
                    ratio_color,
                    theme,
                ))
            }
        } else {
            let items = items
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
                ));

            if compaction.hide_margin_used() {
                items
            } else {
                items.push(vertical_spacer()).push(summary_metric(
                    "Margin Used",
                    self.mask_connected_summary_usd(&summary_values.margin_used_value),
                    theme,
                ))
            }
        }
    }

    fn push_connected_summary_actions<'a>(
        &'a self,
        items: Row<'a, Message>,
        compaction: ConnectedSummaryCompaction,
    ) -> Row<'a, Message> {
        items
            .push(Space::new().width(Fill))
            .push(self.connected_summary_actions_row(compaction))
            .width(Fill)
    }

    fn connected_summary_actions_row<'a>(
        &'a self,
        compaction: ConnectedSummaryCompaction,
    ) -> Row<'a, Message> {
        let mut actions = Row::new()
            .push(self.summary_market_universe_picker())
            .spacing(6)
            .align_y(iced::Alignment::Center);

        if !compaction.hide_display_denomination() {
            actions = actions.push(self.summary_display_denomination_picker());
        }

        actions = actions.push(self.summary_hide_pnl_button());

        if !compaction.hide_sound() {
            actions = actions.push(self.summary_sound_button());
        }

        if !compaction.hide_notifications() {
            actions = actions.push(self.summary_notifications_button());
        }

        actions
            .push(self.summary_layouts_button())
            .push(self.summary_widgets_button())
            .push(self.summary_settings_button())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConnectedSummaryCompaction {
    hidden_priority_count: u8,
}

impl ConnectedSummaryCompaction {
    const fn for_width(width: f32) -> Self {
        let hidden_priority_count = if width < HIDE_MARGIN_USED_WIDTH {
            5
        } else if width < HIDE_MARGIN_RATIO_WIDTH {
            4
        } else if width < HIDE_NOTIFICATION_SELECTOR_WIDTH {
            3
        } else if width < HIDE_SOUND_SELECTOR_WIDTH {
            2
        } else if width < HIDE_DISPLAY_DENOMINATION_SELECTOR_WIDTH {
            1
        } else {
            0
        };

        Self {
            hidden_priority_count,
        }
    }

    const fn hide_display_denomination(self) -> bool {
        self.hidden_priority_count >= 1
    }

    const fn hide_sound(self) -> bool {
        self.hidden_priority_count >= 2
    }

    const fn hide_notifications(self) -> bool {
        self.hidden_priority_count >= 3
    }

    const fn hide_margin_ratio(self) -> bool {
        self.hidden_priority_count >= 4
    }

    const fn hide_margin_used(self) -> bool {
        self.hidden_priority_count >= 5
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
    let value = text(value.to_string())
        .size(13)
        .font(crate::app_fonts::monospace_font());
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
