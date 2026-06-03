mod cells;
mod style;

use self::cells::liquidation_symbol_button;
use self::style::{liquidation_row_color, liquidation_row_style};

use crate::app_state::TradingTerminal;
use crate::feed_state::LiquidationFeedRow;
use crate::feed_views::liquidations::layout::{
    LiquidationFeedRowLayout, METHOD_WIDTH, NUMBER_WIDTH, ROW_SPACING, SIDE_WIDTH, TIME_WIDTH,
};
use crate::helpers;
use crate::message::Message;

use iced::widget::text::Wrapping;
use iced::widget::{Space, container, row, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(super) fn view_liquidation_feed_row(
        &self,
        liq: LiquidationFeedRow,
        now_ms: u64,
        row_layout: LiquidationFeedRowLayout,
    ) -> Element<'static, Message> {
        let theme = self.theme();
        let (color, opacity) = liquidation_row_color(&theme, liq.is_buy, liq.notional);
        let side_str = if liq.is_buy { "BUY" } else { "SELL" };
        let method_label = self.liquidation_method_label(&liq);
        let coin = liq.coin.clone();
        let liquidated_user = liq.liquidated_user.clone();

        let mut row_ui = row![
            text(helpers::format_relative_time(liq.time_ms, now_ms))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text)
                .wrapping(Wrapping::None)
                .width(TIME_WIDTH),
            liquidation_symbol_button(coin, &theme),
        ]
        .spacing(ROW_SPACING)
        .align_y(iced::Alignment::Center);

        if row_layout.show_side {
            row_ui = row_ui.push(
                text(side_str)
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .color(color)
                    .wrapping(Wrapping::None)
                    .width(SIDE_WIDTH),
            );
        }

        if row_layout.show_size {
            row_ui = row_ui.push(
                text(format!("{:.4}", liq.size))
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .color(theme.palette().text)
                    .wrapping(Wrapping::None)
                    .width(NUMBER_WIDTH),
            );
        }

        if row_layout.show_price {
            row_ui = row_ui.push(
                text(self.format_display_price(liq.price))
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .color(theme.palette().text)
                    .wrapping(Wrapping::None)
                    .width(NUMBER_WIDTH),
            );
        }

        row_ui = row_ui.push(
            text(self.format_display_usd_value(liq.notional, 0))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(theme.palette().text)
                .wrapping(Wrapping::None)
                .width(NUMBER_WIDTH),
        );

        if row_layout.show_user {
            row_ui = row_ui.push(self.view_liquidated_user_cell(liquidated_user, &theme));
        }

        if row_layout.show_method {
            row_ui = row_ui.push(Space::new().width(Fill)).push(
                text(method_label)
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .wrapping(Wrapping::None)
                    .width(METHOD_WIDTH),
            );
        }

        container(row_ui)
            .width(Fill)
            .padding([4, 8])
            .style(move |_| liquidation_row_style(color, opacity))
            .into()
    }

    fn liquidation_method_label(&self, liq: &LiquidationFeedRow) -> String {
        if self.liquidation_feed_aggregation_enabled && liq.fill_count > 1 {
            if liq.method.is_empty() {
                format!("x{}", liq.fill_count)
            } else {
                format!("{} x{}", liq.method, liq.fill_count)
            }
        } else {
            liq.method.clone()
        }
    }
}
