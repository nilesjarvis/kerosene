mod row;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::Column;

impl TradingTerminal {
    pub(crate) fn view_liquidation_feed_rows(&self, now_ms: u64) -> Column<'_, Message> {
        let mut list = Column::new().spacing(4);

        for liq in self.visible_liquidation_feed_rows() {
            list = list.push(self.view_liquidation_feed_row(liq, now_ms));
        }

        list
    }
}
