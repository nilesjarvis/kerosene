mod layout;
mod metrics;
mod status;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{container, responsive};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_connected_account_summary(&self) -> Element<'_, Message> {
        if let Some((_, data)) = self.connected_order_account_snapshot() {
            container(responsive(move |size| {
                let theme = self.theme();
                self.view_connected_summary_layout(data, &theme, size.width)
            }))
            .width(Fill)
            .height(Fill)
            .padding([6, 12])
            .center_y(Fill)
            .into()
        } else {
            self.view_connected_account_status()
        }
    }
}
