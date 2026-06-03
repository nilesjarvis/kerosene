mod row;

use crate::app_state::TradingTerminal;
use crate::feed_views::liquidations::layout::LiquidationFeedRowLayout;
use crate::message::Message;
use iced::Fill;
use iced::widget::Column;

impl TradingTerminal {
    pub(in crate::feed_views::liquidations) fn view_liquidation_feed_rows(
        &self,
        now_ms: u64,
        row_layout: LiquidationFeedRowLayout,
    ) -> Column<'_, Message> {
        let mut list = Column::new().spacing(4).width(Fill);

        for liq in self.visible_liquidation_feed_rows() {
            list = list.push(self.view_liquidation_feed_row(liq, now_ms, row_layout));
        }

        list
    }
}
