use crate::app_state::TradingTerminal;
use crate::feed_views::controls::feed_header_text;
use crate::feed_views::tracked_trades::layout::{
    COIN_WIDTH, NUMBER_WIDTH, ROW_SPACING, SIDE_WIDTH, TIME_WIDTH, TrackedTradeRowLayout,
    WALLET_COLUMN_WIDTH,
};
use crate::message::Message;

use iced::widget::{Space, row};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(in crate::feed_views::tracked_trades) fn view_tracked_trades_header(
        &self,
        row_layout: TrackedTradeRowLayout,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let muted_text = theme.extended_palette().background.weak.text;

        let mut header = row![].spacing(ROW_SPACING).align_y(iced::Alignment::Center);

        if row_layout.show_time {
            header = header.push(feed_header_text("Time", muted_text).width(TIME_WIDTH));
        }

        header = header
            .push(feed_header_text("Wallet", muted_text).width(WALLET_COLUMN_WIDTH))
            .push(feed_header_text("Coin", muted_text).width(COIN_WIDTH));

        if row_layout.show_side {
            header = header.push(feed_header_text("Side", muted_text).width(SIDE_WIDTH));
        }

        if row_layout.show_size {
            header = header.push(feed_header_text("Size", muted_text).width(NUMBER_WIDTH));
        }

        if row_layout.show_price {
            header = header.push(feed_header_text("Price", muted_text).width(NUMBER_WIDTH));
        }

        if row_layout.show_notional {
            header = header.push(feed_header_text("Notional", muted_text).width(NUMBER_WIDTH));
        }

        if row_layout.show_pnl {
            header = header.push(feed_header_text("PnL", muted_text).width(NUMBER_WIDTH));
        }

        if row_layout.show_fee {
            header = header.push(feed_header_text("Fee", muted_text).width(NUMBER_WIDTH));
        }

        if row_layout.show_intent {
            header = header
                .push(Space::new().width(Fill))
                .push(feed_header_text("Intent", muted_text));
        }

        header.into()
    }
}
