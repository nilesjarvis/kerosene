mod compaction;
mod metrics_display;
#[cfg(test)]
mod tests;

use super::super::CONNECTED_SUMMARY_ACTION_BREAKPOINT;
use super::metrics::ConnectedSummaryValues;
use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::helpers::vertical_spacer;
use crate::message::Message;
use compaction::ConnectedSummaryCompaction;
use iced::widget::{Row, Space, column, row};
use iced::{Element, Fill, Theme};
use metrics_display::{
    available_color, portfolio_margin_ratio_color, skeleton_metric, summary_metric,
    summary_metric_colored,
};

// ---------------------------------------------------------------------------
// Connected Summary Skeleton
// ---------------------------------------------------------------------------

/// `(label_width, value_width)` for the four core metrics that are always
/// present, in render order: Total Value, Available, Notional Pos, Eff Lev.
/// Portfolio-margin-only columns are omitted since the account mode is unknown
/// until data arrives.
const SKELETON_METRICS: [(f32, f32); 4] = [(56.0, 92.0), (48.0, 76.0), (62.0, 84.0), (40.0, 52.0)];
const SKELETON_PULSE_BASE: f32 = 0.18;
const SKELETON_PULSE_AMP: f32 = 0.10;
const SKELETON_PULSE_OFFSET: f32 = 0.7;

/// Alpha for a value placeholder, breathing within a calm band. The per-metric
/// phase offset turns the breathe into a gentle left-to-right sweep across the
/// row, echoing the chart skeleton's shimmer without a canvas.
fn skeleton_value_alpha(phase: f32, index: usize) -> f32 {
    let wave = 0.5 + 0.5 * (phase - index as f32 * SKELETON_PULSE_OFFSET).sin();
    SKELETON_PULSE_BASE + SKELETON_PULSE_AMP * wave
}

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

    /// Loading placeholder shown on first load (connected, no snapshot yet).
    /// Mirrors [`Self::view_connected_summary_layout`] structurally — same base
    /// row, metric positions, action row and wrap breakpoint — so the flip from
    /// skeleton to real metrics does not shift the layout.
    pub(super) fn view_connected_summary_skeleton(
        &self,
        available_width: f32,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let compaction = ConnectedSummaryCompaction::for_width(available_width);
        let metrics = self.connected_summary_skeleton_row(&theme);

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

    fn connected_summary_skeleton_row<'a>(&'a self, theme: &Theme) -> Row<'a, Message> {
        let phase = self.spinner_phase;
        let mut items = self.connected_summary_base_row();
        for (index, (label_w, value_w)) in SKELETON_METRICS.iter().enumerate() {
            if index > 0 {
                items = items.push(vertical_spacer());
            }
            items = items.push(skeleton_metric(
                *label_w,
                *value_w,
                skeleton_value_alpha(phase, index),
                theme,
            ));
        }
        items.width(Fill)
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
