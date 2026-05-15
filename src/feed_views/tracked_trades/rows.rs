use crate::app_state::TradingTerminal;
use crate::feed_views::tracked_trades::layout::{
    NUMBER_WIDTH, ROW_SPACING, SIDE_WIDTH, TIME_WIDTH, TrackedTradeRowLayout,
};
use crate::helpers;
use crate::message::Message;
use iced::Fill;
use iced::widget::{Column, Space, container, row, text};

mod cells;
mod formatting;
mod style;

impl TradingTerminal {
    pub(in crate::feed_views::tracked_trades) fn view_tracked_trade_rows(
        &self,
        now_ms: u64,
        row_layout: TrackedTradeRowLayout,
    ) -> Column<'_, Message> {
        let theme = self.theme();
        let mut list = Column::new().spacing(2);

        for trade_row in self.visible_tracked_trade_rows() {
            let notional = trade_row.notional;
            let side_color = if trade_row.is_buy {
                theme.palette().success
            } else {
                theme.palette().danger
            };
            let side_str = formatting::tracked_trade_side_label(trade_row.is_buy);
            let pnl_color = style::tracked_trade_pnl_color(&theme, trade_row.closed_pnl);
            let fee_label =
                formatting::tracked_trade_fee_label(trade_row.fee, &trade_row.fee_token);
            let pnl_label = formatting::tracked_trade_pnl_label(trade_row.closed_pnl);
            let intent_text = formatting::tracked_trade_intent_text(
                trade_row.intent,
                &trade_row.dir,
                trade_row.fill_count,
            );

            let mut row_ui = row![
                text(helpers::format_relative_time(trade_row.last_time_ms, now_ms))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(theme.extended_palette().background.weak.text)
                .width(TIME_WIDTH),
                self.view_tracked_trade_wallet_cell(trade_row.address.clone()),
                self.view_tracked_trade_coin_cell(trade_row.coin.clone()),
                text(side_str)
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(side_color)
                    .width(SIDE_WIDTH),
                text(formatting::tracked_trade_size_label(trade_row.size))
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(theme.palette().text)
                    .width(NUMBER_WIDTH),
                text(formatting::tracked_trade_price_label(trade_row.avg_price))
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(theme.palette().text)
                    .width(NUMBER_WIDTH),
                text(formatting::tracked_trade_notional_label(notional))
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(theme.palette().text)
                    .width(NUMBER_WIDTH),
            ]
            .spacing(ROW_SPACING)
            .align_y(iced::Alignment::Center);

            if row_layout.show_pnl {
                row_ui = row_ui.push(
                    text(pnl_label)
                        .size(12)
                        .font(iced::Font::MONOSPACE)
                        .color(pnl_color)
                        .width(NUMBER_WIDTH),
                );
            }

            if row_layout.show_fee {
                row_ui = row_ui.push(
                    text(fee_label)
                        .size(11)
                        .font(iced::Font::MONOSPACE)
                        .color(theme.extended_palette().background.weak.text)
                        .width(NUMBER_WIDTH),
                );
            }

            if row_layout.show_intent {
                row_ui = row_ui.push(Space::new().width(Fill)).push(
                    text(intent_text)
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                );
            }

            list = list.push(
                container(row_ui)
                    .padding([2, 6])
                    .style(style::tracked_trade_row_style(side_color, notional)),
            );
        }

        list
    }
}
