mod connected;
mod controls;
mod disconnected;
mod layout_switcher;
mod menus;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::container;
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
        let content = if self.connected_address.is_none() {
            self.view_disconnected_account_summary()
        } else {
            self.view_connected_account_summary()
        };

        self.view_account_summary_with_menus(content)
    }

    pub(crate) fn pane_grid_min_size(&self) -> f32 {
        PANE_GRID_MIN_SIZE
    }

    pub(crate) fn account_summary_bar_height(&self) -> f32 {
        if self.connected_address.is_none() {
            return ACCOUNT_SUMMARY_WRAPPED_HEIGHT;
        }

        let Some(width) = self.main_window_size.map(|size| size.width) else {
            return ACCOUNT_SUMMARY_DEFAULT_HEIGHT;
        };
        let content_width = (width - ACCOUNT_SUMMARY_HORIZONTAL_PADDING).max(0.0);
        let needs_wrapped_height = if self.account_data.is_some() {
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
}

fn account_summary_bar_style(theme: &Theme) -> container_style::Style {
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
