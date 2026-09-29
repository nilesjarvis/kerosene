use crate::app_state::TradingTerminal;
use crate::feed_views::controls::feed_header_text;
use crate::feed_views::liquidations::layout::{
    COIN_WIDTH, LiquidationFeedRowLayout, METHOD_WIDTH, NUMBER_WIDTH, ROW_SPACING, SIDE_WIDTH,
    TIME_WIDTH, USER_WIDTH,
};
use crate::message::Message;

use iced::widget::{Space, row};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(in crate::feed_views::liquidations) fn view_liquidations_header(
        &self,
        row_layout: LiquidationFeedRowLayout,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let muted_text = theme.extended_palette().background.weak.text;
        let mut header = row![
            feed_header_text("Time", muted_text).width(TIME_WIDTH),
            feed_header_text("Coin", muted_text).width(COIN_WIDTH),
        ]
        .spacing(ROW_SPACING)
        .width(Fill)
        .align_y(iced::Alignment::Center);

        if row_layout.show_side {
            header = header.push(feed_header_text("Side", muted_text).width(SIDE_WIDTH));
        }

        if row_layout.show_size {
            header = header.push(feed_header_text("Size", muted_text).width(NUMBER_WIDTH));
        }

        if row_layout.show_price {
            header = header.push(
                feed_header_text(
                    if self.liquidation_feed_aggregation_enabled {
                        "Avg Px"
                    } else {
                        "Price"
                    },
                    muted_text,
                )
                .width(NUMBER_WIDTH),
            );
        }

        header = header.push(feed_header_text("Notional", muted_text).width(NUMBER_WIDTH));

        if row_layout.show_user {
            header = header.push(feed_header_text("User", muted_text).width(USER_WIDTH));
        }

        if row_layout.show_method {
            header = header
                .push(Space::new().width(Fill))
                .push(feed_header_text("Method", muted_text).width(METHOD_WIDTH));
        }

        header.into()
    }
}
