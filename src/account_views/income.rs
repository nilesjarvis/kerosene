mod content;
mod projection;
mod rows;
mod status;

use crate::account::AccountData;
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
            .account_data
            .as_ref()
            .is_some_and(AccountData::is_portfolio_margin);

        if !is_pm {
            return container(self.view_income_unavailable())
                .width(Fill)
                .height(Fill)
                .padding(10)
                .into();
        }

        if self.income.loading && self.income.data.is_none() {
            return container(self.view_income_loading())
                .width(Fill)
                .height(Fill)
                .padding(10)
                .into();
        }

        let Some(data) = &self.income.data else {
            return container(self.view_income_empty())
                .width(Fill)
                .height(Fill)
                .padding(10)
                .into();
        };

        container(self.view_income_data(data))
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }
}
