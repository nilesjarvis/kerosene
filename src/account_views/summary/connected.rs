mod layout;
mod metrics;
mod status;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container;
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_connected_account_summary(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let addr = self.connected_address.as_deref().unwrap_or("");
        let account_label = self.connected_account_label(addr);

        if let Some(data) = &self.account_data {
            let summary_values = self.connected_summary_values(data);
            let items = self.connected_summary_base_row(addr, &account_label, &theme);
            let items = self.push_connected_summary_metrics(items, data, &summary_values, &theme);
            let items = self.push_connected_summary_actions(items);

            container(items)
                .width(Fill)
                .height(Fill)
                .padding([2, 12])
                .center_y(Fill)
                .into()
        } else {
            self.view_connected_account_status(account_label)
        }
    }
}
