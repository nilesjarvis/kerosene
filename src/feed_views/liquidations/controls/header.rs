use crate::app_state::TradingTerminal;
use crate::feed_views::liquidations::layout::{
    COIN_WIDTH, LiquidationFeedRowLayout, METHOD_WIDTH, NUMBER_WIDTH, ROW_SPACING, SIDE_WIDTH,
    TIME_WIDTH, USER_WIDTH,
};
use crate::message::Message;

use iced::widget::text::Wrapping;
use iced::widget::{Space, row, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(in crate::feed_views::liquidations) fn view_liquidations_header(
        &self,
        row_layout: LiquidationFeedRowLayout,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let muted_text = theme.extended_palette().background.weak.text;
        let mut header = row![
            text("Time")
                .size(11)
                .color(muted_text)
                .wrapping(Wrapping::None)
                .width(TIME_WIDTH),
            text("Coin")
                .size(11)
                .color(muted_text)
                .wrapping(Wrapping::None)
                .width(COIN_WIDTH),
        ]
        .spacing(ROW_SPACING)
        .width(Fill)
        .align_y(iced::Alignment::Center);

        if row_layout.show_side {
            header = header.push(
                text("Side")
                    .size(11)
                    .color(muted_text)
                    .wrapping(Wrapping::None)
                    .width(SIDE_WIDTH),
            );
        }

        if row_layout.show_size {
            header = header.push(
                text("Size")
                    .size(11)
                    .color(muted_text)
                    .wrapping(Wrapping::None)
                    .width(NUMBER_WIDTH),
            );
        }

        if row_layout.show_price {
            header = header.push(
                text(if self.liquidation_feed_aggregation_enabled {
                    "Avg Px"
                } else {
                    "Price"
                })
                .size(11)
                .color(muted_text)
                .wrapping(Wrapping::None)
                .width(NUMBER_WIDTH),
            );
        }

        header = header.push(
            text("Notional")
                .size(11)
                .color(muted_text)
                .wrapping(Wrapping::None)
                .width(NUMBER_WIDTH),
        );

        if row_layout.show_user {
            header = header.push(
                text("User")
                    .size(11)
                    .color(muted_text)
                    .wrapping(Wrapping::None)
                    .width(USER_WIDTH),
            );
        }

        if row_layout.show_method {
            header = header.push(Space::new().width(Fill)).push(
                text("Method")
                    .size(11)
                    .color(muted_text)
                    .wrapping(Wrapping::None)
                    .width(METHOD_WIDTH),
            );
        }

        header.into()
    }
}
