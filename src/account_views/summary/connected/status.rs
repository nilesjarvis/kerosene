use crate::app_state::TradingTerminal;
use crate::helpers::{label_value, vertical_spacer};
use crate::message::Message;
use iced::widget::{container, row, text};
use iced::{Element, Fill};

// ---------------------------------------------------------------------------
// Connected Account Status
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_connected_account_status(
        &self,
        account_label: String,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let loading_label = if self.account_loading {
            "Loading account..."
        } else {
            "No account data"
        };
        let account_warning = self.account_data.as_ref().and_then(|data| {
            (!data.completeness.is_complete())
                .then(|| data.completeness.warning_summary())
                .flatten()
        });
        let account_status_text = self
            .account_error
            .as_deref()
            .or(account_warning.as_deref())
            .unwrap_or(loading_label)
            .to_string();
        let account_status_color = if self.account_error.is_some() {
            theme.palette().danger
        } else if account_warning.is_some() {
            theme.palette().warning
        } else {
            theme.extended_palette().background.weak.text
        };
        let status_widget: Element<'_, Message> = if self.account_loading {
            row![
                self.summary_account_picker(),
                self.summary_add_account_button(),
                self.summary_forget_ghost_button(),
                vertical_spacer(),
                self.view_spinner(16),
                text(account_status_text)
                    .size(11)
                    .color(account_status_color)
                    .width(Fill),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .into()
        } else {
            row![
                self.summary_account_picker(),
                self.summary_add_account_button(),
                self.summary_forget_ghost_button(),
                vertical_spacer(),
                text(account_status_text)
                    .size(11)
                    .color(account_status_color)
                    .width(Fill),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .into()
        };
        let items = row![
            label_value("Account", account_label),
            vertical_spacer(),
            status_widget,
            vertical_spacer(),
            self.summary_widgets_button(),
            self.summary_settings_button(),
            self.summary_disconnect_button(),
        ]
        .spacing(16)
        .align_y(iced::Alignment::Center);

        container(items)
            .width(Fill)
            .height(Fill)
            .padding([2, 12])
            .center_y(Fill)
            .into()
    }
}
