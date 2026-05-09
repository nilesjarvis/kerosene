use crate::account_metrics::format_signed_usd_value;
use crate::app_state::TradingTerminal;
use crate::helpers::{self, format_usd};
use crate::message::Message;
use iced::Fill;
use iced::widget::{Column, Space, container, row, text};

mod cells;
mod formatting;
mod style;

impl TradingTerminal {
    pub(crate) fn view_tracked_trade_rows(&self, now_ms: u64) -> Column<'_, Message> {
        let theme = self.theme();
        let mut list = Column::new().spacing(4);

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
            let intent_text = formatting::tracked_trade_intent_text(
                trade_row.intent,
                &trade_row.dir,
                trade_row.fill_count,
            );

            let row_ui = row![
                text(helpers::format_relative_time(
                    trade_row.last_time_ms,
                    now_ms
                ))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(theme.extended_palette().background.weak.text)
                .width(60),
                self.view_tracked_trade_wallet_cell(trade_row.address.clone()),
                self.view_tracked_trade_coin_cell(trade_row.coin.clone()),
                text(side_str)
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(side_color)
                    .width(50),
                text(format!("{:.4}", trade_row.size))
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(theme.palette().text)
                    .width(80),
                text(format!("{:.4}", trade_row.avg_price))
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(theme.palette().text)
                    .width(80),
                text(format_usd(&format!("{:.0}", notional)))
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(theme.palette().text)
                    .width(80),
                text(format_signed_usd_value(trade_row.closed_pnl))
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(pnl_color)
                    .width(80),
                text(fee_label)
                    .size(11)
                    .font(iced::Font::MONOSPACE)
                    .color(theme.extended_palette().background.weak.text)
                    .width(80),
                Space::new().width(Fill),
                text(intent_text)
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);

            list = list.push(
                container(row_ui)
                    .padding([4, 8])
                    .style(style::tracked_trade_row_style(side_color, notional)),
            );
        }

        list
    }
}
