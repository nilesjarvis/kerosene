mod connected;
mod controls;
mod disconnected;
mod layout_switcher;
mod menus;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Element;

pub(crate) const CONNECTED_SUMMARY_ACTION_BREAKPOINT: f32 = 1180.0;
pub(crate) const CONNECTED_STATUS_ACTION_BREAKPOINT: f32 = 820.0;

const ACCOUNT_SUMMARY_DEFAULT_PANE_MIN_SIZE: f32 = 50.0;
const ACCOUNT_SUMMARY_WRAPPED_PANE_MIN_SIZE: f32 = 104.0;
const ACCOUNT_SUMMARY_HORIZONTAL_PADDING: f32 = 24.0;

impl TradingTerminal {
    pub(crate) fn view_account_summary(&self) -> Element<'_, Message> {
        let content = if self.connected_address.is_none() {
            self.view_disconnected_account_summary()
        } else {
            self.view_connected_account_summary()
        };

        self.view_account_summary_with_menus(content)
    }

    pub(crate) fn account_summary_pane_min_size(&self) -> f32 {
        let Some(width) = self.main_window_size.map(|size| size.width) else {
            return ACCOUNT_SUMMARY_DEFAULT_PANE_MIN_SIZE;
        };
        let content_width = (width - ACCOUNT_SUMMARY_HORIZONTAL_PADDING).max(0.0);
        let needs_wrapped_height = if self.connected_address.is_none() {
            false
        } else if self.account_data.is_some() {
            content_width < CONNECTED_SUMMARY_ACTION_BREAKPOINT
        } else {
            content_width < CONNECTED_STATUS_ACTION_BREAKPOINT
        };

        if needs_wrapped_height {
            ACCOUNT_SUMMARY_WRAPPED_PANE_MIN_SIZE
        } else {
            ACCOUNT_SUMMARY_DEFAULT_PANE_MIN_SIZE
        }
    }
}
