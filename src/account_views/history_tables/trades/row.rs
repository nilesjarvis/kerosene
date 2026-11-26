use super::aggregation::{HistoryTrade, fee_currency};
use crate::account_views::history::format_history_time_millis;
use crate::account_views::history_tables::numbers::{
    format_history_display_usd, invalid_history_data,
};
use crate::account_views::history_tables::style::history_signed_value_color;
use crate::app_state::TradingTerminal;
use crate::helpers::{self, optional_value_color, trim_decimal_zeros};
use crate::message::Message;
use iced::widget::text::Wrapping;
use iced::widget::{Row, Space, row, text};
use iced::{Color, Element, Fill, Theme};

pub(super) fn view_trade_history_header(theme: &Theme, aggregate: bool) -> Row<'static, Message> {
    let header_txt = |s: &'static str| {
        text(s)
            .size(11)
            .color(theme.extended_palette().background.weak.text)
            .width(Fill)
            .wrapping(Wrapping::None)
    };

    let header = row![
        header_txt("Time"),
        header_txt("Symbol"),
        header_txt("Side"),
        header_txt("Dir"),
        header_txt(if aggregate { "Avg price" } else { "Price" }),
        header_txt("Size"),
        header_txt("PnL"),
        header_txt("Fee"),
    ]
    .spacing(4);
    if aggregate {
        header.push(header_txt("Fills").width(32))
    } else {
        header
    }
}

impl TradingTerminal {
    pub(super) fn view_trade_history_row<'a>(
        &'a self,
        trade: HistoryTrade<'a>,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let fill = trade.first;
        let (side_str, side_color) = trade_side_display(&fill.side, theme);

        let time_str = format_history_time_millis(fill.time);

        let amounts = trade.amounts();
        let pnl = amounts.pnl;
        let fee = amounts.fee;
        let display_amount = |wire: &str, value: Option<f64>| match value {
            None => invalid_history_data(),
            Some(_) if trade.count() == 1 => wire.to_owned(),
            Some(value) => trim_decimal_zeros(format!("{value:.8}")),
        };
        let weak_color = theme.extended_palette().background.weak.text;
        let invalid_color = theme.palette().warning;
        let pnl_color = history_signed_value_color(pnl, theme);
        let fee_color = optional_value_color(fee, weak_color, invalid_color);
        let denomination = self.display_denomination_context();

        let pnl_display = if self.hide_pnl {
            denomination.hidden_mask()
        } else {
            format_history_display_usd(&denomination, pnl, 2)
        };
        let fee_display =
            history_fee_display(&denomination, fee, fill.fee_token.as_deref(), self.hide_pnl);

        let mut coin_content = row![];
        if let Some(icon) = helpers::symbol_icon(&fill.coin, 14, theme.palette().text) {
            coin_content = coin_content.push(icon).push(Space::new().width(4.0));
        }
        coin_content = coin_content
            .push(
                text(self.display_coin_for_journal(&fill.coin))
                    .size(12)
                    .wrapping(Wrapping::None),
            )
            .align_y(iced::Alignment::Center);

        let row = row![
            text(time_str).size(12).width(Fill).wrapping(Wrapping::None),
            coin_content.width(Fill),
            text(side_str)
                .size(12)
                .color(side_color)
                .width(Fill)
                .wrapping(Wrapping::None),
            text(trade.direction())
                .size(12)
                .width(Fill)
                .wrapping(Wrapping::None),
            text(display_amount(&fill.px, amounts.price))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(optional_value_color(
                    amounts.price,
                    weak_color,
                    invalid_color
                ))
                .width(Fill)
                .wrapping(Wrapping::None),
            text(display_amount(&fill.sz, amounts.size))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(optional_value_color(
                    amounts.size,
                    weak_color,
                    invalid_color
                ))
                .width(Fill)
                .wrapping(Wrapping::None),
            text(pnl_display)
                .size(12)
                .color(pnl_color)
                .width(Fill)
                .wrapping(Wrapping::None),
            text(fee_display)
                .size(12)
                .color(fee_color)
                .width(Fill)
                .wrapping(Wrapping::None),
        ]
        .spacing(4);
        if self.aggregate_trade_history {
            row.push(text(trade.count()).size(12).width(32)).into()
        } else {
            row.into()
        }
    }
}

fn trade_side_display(side: &str, theme: &Theme) -> (&'static str, Color) {
    match side {
        "B" => ("+ Buy", theme.palette().success),
        "A" => ("- Sell", theme.palette().danger),
        _ => ("Invalid", theme.palette().warning),
    }
}

/// Spot buy fees are charged in the base token (sells and perp fees in
/// USDC), so a non-USDC fee is displayed as a token quantity with its token
/// symbol instead of being passed off as a dollar amount.
fn history_fee_display(
    denomination: &crate::denomination::DisplayDenominationContext,
    fee: Option<f64>,
    fee_token: Option<&str>,
    hide_pnl: bool,
) -> String {
    if hide_pnl {
        return denomination.hidden_mask();
    }
    let Some(fee) = fee else {
        return invalid_history_data();
    };
    match fee_currency(fee_token) {
        "USDC" if fee < 0.0 => format!("+{}", denomination.format_value(fee.abs(), 2)),
        "USDC" => format!("-{}", denomination.format_value(fee, 2)),
        token => format_token_fee(fee, token),
    }
}

/// Fees show as costs: a paid fee is negative, a rebate positive. Eight
/// decimals cover Hyperliquid token precision; trailing zeros are trimmed.
fn format_token_fee(fee: f64, token: &str) -> String {
    let sign = if fee > 0.0 {
        "-"
    } else if fee < 0.0 {
        "+"
    } else {
        ""
    };
    let amount = trim_decimal_zeros(format!("{:.8}", fee.abs()));
    format!("{sign}{amount} {token}")
}

#[cfg(test)]
mod tests;
