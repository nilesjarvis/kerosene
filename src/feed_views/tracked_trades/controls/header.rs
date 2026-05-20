use crate::app_state::TradingTerminal;
use crate::feed_views::tracked_trades::layout::{
    COIN_WIDTH, NUMBER_WIDTH, ROW_SPACING, SIDE_WIDTH, TIME_WIDTH, TrackedTradeRowLayout,
    WALLET_COLUMN_WIDTH,
};
use crate::message::Message;

use iced::widget::text::Wrapping;
use iced::widget::{Space, row, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(in crate::feed_views::tracked_trades) fn view_tracked_trades_header(
        &self,
        row_layout: TrackedTradeRowLayout,
    ) -> Element<'_, Message> {
        let theme = self.theme();

        let mut header = row![].spacing(ROW_SPACING).align_y(iced::Alignment::Center);

        if row_layout.show_time {
            header = header.push(
                text("Time")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .wrapping(Wrapping::None)
                    .width(TIME_WIDTH),
            );
        }

        header = header
            .push(
                text("Wallet")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .wrapping(Wrapping::None)
                    .width(WALLET_COLUMN_WIDTH),
            )
            .push(
                text("Coin")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .wrapping(Wrapping::None)
                    .width(COIN_WIDTH),
            );

        if row_layout.show_side {
            header = header.push(
                text("Side")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .wrapping(Wrapping::None)
                    .width(SIDE_WIDTH),
            );
        }

        if row_layout.show_size {
            header = header.push(
                text("Size")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .wrapping(Wrapping::None)
                    .width(NUMBER_WIDTH),
            );
        }

        if row_layout.show_price {
            header = header.push(
                text("Price")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .wrapping(Wrapping::None)
                    .width(NUMBER_WIDTH),
            );
        }

        if row_layout.show_notional {
            header = header.push(
                text("Notional")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .wrapping(Wrapping::None)
                    .width(NUMBER_WIDTH),
            );
        }

        if row_layout.show_pnl {
            header = header.push(
                text("PnL")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .wrapping(Wrapping::None)
                    .width(NUMBER_WIDTH),
            );
        }

        if row_layout.show_fee {
            header = header.push(
                text("Fee")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .wrapping(Wrapping::None)
                    .width(NUMBER_WIDTH),
            );
        }

        if row_layout.show_intent {
            header = header.push(Space::new().width(Fill)).push(
                text("Intent")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .wrapping(Wrapping::None),
            );
        }

        header.into()
    }
}
