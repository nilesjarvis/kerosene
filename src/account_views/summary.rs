mod connected;
mod controls;
mod disconnected;
mod layout_switcher;
mod menus;

use crate::account_state::ActiveAccountSource;
use crate::app_state::TradingTerminal;
use crate::helpers::{format_usd, vertical_spacer};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Fill, Length, Theme};

pub(crate) const CONNECTED_SUMMARY_ACTION_BREAKPOINT: f32 = 1180.0;
pub(crate) const CONNECTED_STATUS_ACTION_BREAKPOINT: f32 = 820.0;

const ACCOUNT_SUMMARY_DEFAULT_HEIGHT: f32 = 54.0;
const ACCOUNT_SUMMARY_WRAPPED_HEIGHT: f32 = 104.0;
const ACCOUNT_SUMMARY_HORIZONTAL_PADDING: f32 = 24.0;
const ACCOUNT_SUMMARY_BORDER_WIDTH: f32 = 1.0;
const PANE_GRID_MIN_SIZE: f32 = 50.0;

impl TradingTerminal {
    pub(crate) fn view_account_summary_bar(&self) -> Element<'_, Message> {
        container(self.view_account_summary())
            .width(Fill)
            .height(Length::Fixed(self.account_summary_bar_height()))
            .style(account_summary_bar_style)
            .into()
    }

    pub(crate) fn view_account_summary(&self) -> Element<'_, Message> {
        let content = if self.active_account_source == ActiveAccountSource::Schwab {
            self.view_schwab_account_summary()
        } else if self.account_summary_is_connected_or_connecting() {
            self.view_connected_account_summary()
        } else {
            self.view_disconnected_account_summary()
        };

        self.view_account_summary_with_menus(content)
    }

    /// Whether the connected summary (metrics or its loading skeleton) should be
    /// shown rather than the disconnected add-account form. Covers the transient
    /// window during an account switch / boot where `connected_address` is still
    /// `None` but a connect is already loading or in flight.
    pub(crate) fn account_summary_is_connected_or_connecting(&self) -> bool {
        if self.active_account_source == ActiveAccountSource::Schwab {
            return self.schwab.selected_account_hash.is_some();
        }
        self.connected_address.is_some() || self.account_summary_is_loading()
    }

    /// Whether the connected summary should render its loading skeleton: either
    /// account data is loading, or a connect was just dispatched (switch / boot)
    /// and has not yet been processed.
    pub(crate) fn account_summary_is_loading(&self) -> bool {
        self.account_loading || self.account_connect_pending
    }

    pub(crate) fn pane_grid_min_size(&self) -> f32 {
        PANE_GRID_MIN_SIZE
    }

    pub(crate) fn account_summary_bar_height(&self) -> f32 {
        if self.active_account_source == ActiveAccountSource::Schwab {
            return ACCOUNT_SUMMARY_WRAPPED_HEIGHT;
        }
        if !self.account_summary_is_connected_or_connecting() {
            return ACCOUNT_SUMMARY_WRAPPED_HEIGHT;
        }

        let Some(width) = self.main_window_size.map(|size| size.width) else {
            return ACCOUNT_SUMMARY_DEFAULT_HEIGHT;
        };
        let content_width = (width - ACCOUNT_SUMMARY_HORIZONTAL_PADDING).max(0.0);
        // While loading (skeleton) and once populated, both render the full
        // metrics layout, so use the metrics breakpoint for both — this makes
        // the loading height pre-match the populated height so the no-data ->
        // data flip never jumps. The narrower status breakpoint only applies to
        // the genuine non-loading no-data / error message.
        let needs_wrapped_height = if self.connected_order_account_snapshot().is_some()
            || self.account_summary_is_loading()
        {
            content_width < CONNECTED_SUMMARY_ACTION_BREAKPOINT
        } else {
            content_width < CONNECTED_STATUS_ACTION_BREAKPOINT
        };

        if needs_wrapped_height {
            ACCOUNT_SUMMARY_WRAPPED_HEIGHT
        } else {
            ACCOUNT_SUMMARY_DEFAULT_HEIGHT
        }
    }

    fn view_schwab_account_summary(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let summary = self.schwab.selected_account_summary();
        let status = self
            .schwab
            .status
            .as_ref()
            .map(|(message, is_error)| {
                text(message).size(10).color(if *is_error {
                    theme.palette().danger
                } else {
                    theme.extended_palette().background.weak.text
                })
            })
            .unwrap_or_else(|| {
                text("Schwab account selected")
                    .size(10)
                    .color(theme.extended_palette().background.weak.text)
            });

        let top_row = row![
            self.summary_account_picker(),
            vertical_spacer(),
            column![
                text("Schwab Brokerage")
                    .size(13)
                    .color(theme.palette().text),
                status,
            ]
            .spacing(2)
            .width(Fill),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        let metric = |label: &'static str, value: String| {
            column![
                text(label)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
                text(value).size(12).color(theme.palette().text),
            ]
            .spacing(2)
        };

        let money = |value: Option<f64>| {
            value
                .filter(|value| value.is_finite())
                .map(|value| format_usd(&format!("{value:.2}")))
                .unwrap_or_else(|| "--".to_string())
        };
        let refresh = button(
            text(if self.schwab.loading() {
                "Loading"
            } else {
                "Refresh"
            })
            .size(11),
        )
        .on_press(Message::SchwabAccountsRefresh)
        .padding([4, 10]);

        let metrics_row = row![
            metric(
                "Total Value",
                money(summary.and_then(|account| account.liquidation_value))
            ),
            vertical_spacer(),
            metric(
                "Cash",
                money(summary.and_then(|account| account.cash_balance))
            ),
            vertical_spacer(),
            metric(
                "Buying Power",
                money(summary.and_then(|account| account.buying_power))
            ),
            vertical_spacer(),
            metric(
                "Positions",
                summary
                    .map(|account| account.positions.len().to_string())
                    .unwrap_or_else(|| "--".to_string())
            ),
            Space::new().width(Fill),
            refresh,
            self.summary_layouts_button(),
            self.summary_widgets_button(),
            self.summary_settings_button(),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        container(column![top_row, metrics_row].spacing(6))
            .width(Fill)
            .height(Fill)
            .padding([6, 12])
            .center_y(Fill)
            .into()
    }
}

pub(crate) fn account_summary_bar_style(theme: &Theme) -> container_style::Style {
    let mut border_color = theme.extended_palette().background.strong.text;
    border_color.a = 0.10;

    container_style::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        text_color: Some(theme.palette().text),
        border: iced::Border {
            width: ACCOUNT_SUMMARY_BORDER_WIDTH,
            color: border_color,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connected_terminal(content_width: Option<f32>) -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        terminal.main_window_size =
            content_width.map(|w| iced::Size::new(w + ACCOUNT_SUMMARY_HORIZONTAL_PADDING, 800.0));
        terminal
    }

    #[test]
    fn loading_bar_height_pre_matches_populated_metrics_breakpoint() {
        // While loading (no snapshot yet) the bar must already size to the
        // metrics breakpoint so the no-data -> data flip never changes height.
        // Below 1180 this is WRAPPED; the old status breakpoint (820) would
        // have reported DEFAULT here and caused a jump when data arrived.
        let mut narrow = connected_terminal(Some(1000.0));
        narrow.account_loading = true;
        assert_eq!(
            narrow.account_summary_bar_height(),
            ACCOUNT_SUMMARY_WRAPPED_HEIGHT
        );

        let mut wide = connected_terminal(Some(1200.0));
        wide.account_loading = true;
        assert_eq!(
            wide.account_summary_bar_height(),
            ACCOUNT_SUMMARY_DEFAULT_HEIGHT
        );
    }

    #[test]
    fn non_loading_status_keeps_narrow_status_breakpoint() {
        // The genuine "No account data" / error state (connected, not loading,
        // no snapshot) keeps the narrower status breakpoint, so it does not get
        // an over-tall bar at mid widths.
        let terminal = connected_terminal(Some(1000.0));
        assert!(!terminal.account_loading);
        assert_eq!(
            terminal.account_summary_bar_height(),
            ACCOUNT_SUMMARY_DEFAULT_HEIGHT
        );
    }

    #[test]
    fn disconnected_bar_uses_wrapped_height() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = None;
        assert_eq!(
            terminal.account_summary_bar_height(),
            ACCOUNT_SUMMARY_WRAPPED_HEIGHT
        );
    }

    #[test]
    fn connected_without_window_size_uses_default_height() {
        let mut terminal = connected_terminal(None);
        terminal.account_loading = true;
        assert_eq!(
            terminal.account_summary_bar_height(),
            ACCOUNT_SUMMARY_DEFAULT_HEIGHT
        );
    }
}
