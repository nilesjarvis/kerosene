mod cells;
mod style;

use self::cells::liquidation_symbol_button;
use self::style::{liquidation_row_color, liquidation_row_style};

use crate::app_state::TradingTerminal;
use crate::feed_state::LiquidationFeedRow;
use crate::helpers::{self, format_usd};
use crate::message::Message;

use iced::widget::{Space, container, row, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(super) fn view_liquidation_feed_row(
        &self,
        liq: LiquidationFeedRow,
        now_ms: u64,
    ) -> Element<'static, Message> {
        let theme = self.theme();
        let (color, opacity) = liquidation_row_color(&theme, liq.is_buy, liq.notional);
        let side_str = if liq.is_buy { "BUY" } else { "SELL" };
        let method_label = self.liquidation_method_label(&liq);
        let coin = liq.coin.clone();
        let liquidated_user = liq.liquidated_user.clone();

        let row_ui = row![
            text(helpers::format_relative_time(liq.time_ms, now_ms))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(theme.extended_palette().background.weak.text)
                .width(60),
            liquidation_symbol_button(coin, &theme),
            text(side_str)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(color)
                .width(50),
            text(format!("{:.4}", liq.size))
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(theme.palette().text)
                .width(80),
            text(format!("{:.4}", liq.price))
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(theme.palette().text)
                .width(80),
            text(format_usd(&format!("{:.0}", liq.notional)))
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(theme.palette().text)
                .width(80),
            self.view_liquidated_user_cell(liquidated_user, &theme),
            Space::new().width(Fill),
            text(method_label)
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        container(row_ui)
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
