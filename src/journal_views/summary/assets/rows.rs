use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Row, Space, row, text};
use iced::{Element, Fill, Theme};

pub(super) fn journal_asset_table_header(theme: &Theme) -> Row<'static, Message> {
    row![
        text("Asset")
            .size(11)
            .color(theme.extended_palette().background.weak.text)
            .width(100),
        text("Trades")
            .size(11)
            .color(theme.extended_palette().background.weak.text)
            .width(80),
        text("Fees Paid")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        Space::new().width(Fill),
        text("Total PnL")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
    ]
    .align_y(iced::Alignment::Center)
}

impl TradingTerminal {
    pub(super) fn view_journal_asset_table_row<'a>(
        &'a self,
        coin: &str,
        count: usize,
        pnl: f64,
        fees: f64,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let display_coin = self.display_coin_for_journal(coin);
        let pnl_color = journal_asset_pnl_color(pnl, theme);
        let denomination = self.display_denomination_context();

        row![
            text(display_coin)
                .size(12)
                .color(theme.palette().text)
                .width(100),
            text(format!("{}", count))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(theme.palette().text)
                .width(80),
            text(denomination.format_value(fees, 2))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(theme.palette().danger),
            Space::new().width(Fill),
            text(denomination.format_value(pnl, 2))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(pnl_color),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }

    pub(super) fn view_journal_asset_compact_row<'a>(
        &'a self,
        coin: &str,
        count: usize,
        pnl: f64,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let display_coin = self.display_coin_for_journal(coin);
        let pnl_color = journal_asset_pnl_color(pnl, theme);
        let denomination = self.display_denomination_context();

        row![
            text(display_coin)
                .size(12)
                .color(theme.palette().text)
                .width(60),
            text(format!("{} trades", count))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text),
            Space::new().width(Fill),
            text(denomination.format_value(pnl, 2))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(pnl_color),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }
}

fn journal_asset_pnl_color(pnl: f64, theme: &Theme) -> iced::Color {
    if pnl > 0.0 {
        theme.palette().success
    } else if pnl < 0.0 {
        theme.palette().danger
    } else {
        theme.palette().text
    }
}
