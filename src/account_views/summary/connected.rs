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
            wrap_connected_summary(responsive(move |size| {
                let theme = self.theme();
                self.view_connected_summary_layout(data, &theme, size.width)
            }))
        } else if self.account_loading {
            wrap_connected_summary(responsive(move |size| {
                self.view_connected_summary_skeleton(size.width)
            }))
        } else {
            self.view_connected_account_status()
        }
    }
}

/// Shared container chrome for the connected summary content (populated metrics
/// and the loading skeleton), so both share identical padding/alignment and the
/// flip between them never shifts the layout.
fn wrap_connected_summary<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Fill)
        .height(Fill)
        .padding([6, 12])
        .center_y(Fill)
        .into()
}
