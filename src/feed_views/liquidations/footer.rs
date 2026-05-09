use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Element;
use iced::widget::column;

mod chart;
mod summary;

impl TradingTerminal {
    pub(crate) fn view_liquidations_bottom_content(&self, now_ms: u64) -> Element<'_, Message> {
        let mut bottom_content = column![].spacing(8);

        if self.liquidation_chart_enabled {
            bottom_content = bottom_content.push(self.view_liquidations_chart(now_ms));
        }

        if self.liquidation_summary_enabled {
            bottom_content = bottom_content.push(self.view_liquidations_summary(now_ms));
        }

        bottom_content.into()
    }
}
