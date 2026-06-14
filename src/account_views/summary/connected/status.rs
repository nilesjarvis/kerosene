use super::super::CONNECTED_STATUS_ACTION_BREAKPOINT;
use crate::app_state::TradingTerminal;
use crate::helpers::vertical_spacer;
use crate::message::Message;
use iced::widget::{Row, Space, column, container, responsive, row, text};
use iced::{Element, Fill};

// ---------------------------------------------------------------------------
// Connected Account Status
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_connected_account_status(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_connected_account_status_layout(size.width))
            .width(Fill)
            .height(Fill)
            .into()
    }

    // Non-loading status only: the loading state routes to the skeleton in
    // `view_connected_account_summary`, so this renders the genuine no-data /
    // error / warning message.
    fn view_connected_account_status_layout(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let ready_status = self.connected_order_account_snapshot().map(|(_, data)| {
            let scope = data
                .fetch_scope
                .selected_hip3_dex()
                .map(|dex| format!("HIP-3 {dex}"))
                .unwrap_or_else(|| "All markets".to_string());
            format!("{scope} refresh (~{} API wt)", data.request_weight_estimate)
        });
        let account_warning = self
            .connected_order_account_snapshot()
            .and_then(|(_, data)| {
                (!data.completeness.is_complete())
                    .then(|| data.completeness.warning_summary())
                    .flatten()
            });
        let account_status_text = self
            .account_error
            .as_deref()
            .or(account_warning.as_deref())
            .or(ready_status.as_deref())
            .unwrap_or("No account data")
            .to_string();
        let account_status_color = if self.account_error.is_some() {
            theme.palette().danger
        } else if account_warning.is_some() {
            theme.palette().warning
        } else {
            theme.extended_palette().background.weak.text
        };
        let status_widget = row![
            self.summary_account_picker(),
            vertical_spacer(),
            text(account_status_text)
                .size(11)
                .color(account_status_color)
                .width(Fill),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);
        let actions = self.connected_status_actions_row();
        let items: Element<'_, Message> = if available_width < CONNECTED_STATUS_ACTION_BREAKPOINT {
            column![
                status_widget,
                row![Space::new().width(Fill), actions]
                    .width(Fill)
                    .align_y(iced::Alignment::Center),
            ]
            .spacing(6)
            .width(Fill)
            .into()
        } else {
            row![status_widget, vertical_spacer(), actions]
                .spacing(16)
                .align_y(iced::Alignment::Center)
                .width(Fill)
                .into()
        };

        container(items)
            .width(Fill)
            .height(Fill)
            .padding([6, 12])
            .center_y(Fill)
            .into()
    }

    fn connected_status_actions_row(&self) -> Row<'_, Message> {
        row![
            self.summary_market_universe_picker(),
            self.summary_layouts_button(),
            self.summary_widgets_button(),
            self.summary_settings_button(),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
    }
}
