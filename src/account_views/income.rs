mod content;
mod projection;
mod rows;
mod status;
mod tabs;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container;
use iced::{Element, Fill};

// ---------------------------------------------------------------------------
// Portfolio Margin Income View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_income(&self) -> Element<'_, Message> {
        let is_pm = self
            .connected_order_account_snapshot()
            .is_some_and(|(_, data)| data.is_portfolio_margin());

        let content = if !is_pm {
            self.view_income_unavailable().into()
        } else if self.income.refresh.loading && self.income.data.is_none() {
            self.view_income_loading().into()
        } else if let Some(data) = &self.income.data {
            self.view_income_data(data)
        } else {
            self.view_income_empty().into()
        };

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }
}
