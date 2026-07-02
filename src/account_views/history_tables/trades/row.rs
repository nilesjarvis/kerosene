use crate::account;
use crate::account_views::history::format_history_time_millis;
use crate::account_views::history_tables::numbers::{
    format_history_display_usd, invalid_history_data, parse_history_number,
    valid_history_wire_value,
};
use crate::account_views::history_tables::style::history_signed_value_color;
use crate::app_state::TradingTerminal;
use crate::helpers::{self, optional_value_color, trim_decimal_zeros};
use crate::message::Message;
use iced::widget::text::Wrapping;
use iced::widget::{Row, Space, row, text};
use iced::{Color, Element, Fill, Theme};

pub(super) fn view_trade_history_header(theme: &Theme) -> Row<'static, Message> {
    let header_txt = |s: &'static str| {
        text(s)
            .size(11)
            .color(theme.extended_palette().background.weak.text)
            .width(Fill)
            .wrapping(Wrapping::None)
    };

    row![
        header_txt("Time"),
        header_txt("Symbol"),
        header_txt("Side"),
        header_txt("Dir"),
        header_txt("Price"),
        header_txt("Size"),
        header_txt("PnL"),
        header_txt("Fee"),
    ]
    .spacing(4)
}

impl TradingTerminal {
    pub(super) fn view_trade_history_row<'a>(
        &'a self,
        fill: &'a account::UserFill,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let (side_str, side_color) = trade_side_display(&fill.side, theme);

        let time_str = format_history_time_millis(fill.time);

        let pnl = parse_history_number(&fill.closed_pnl);
        let fee = parse_history_number(&fill.fee);
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

        row![
            text(time_str).size(12).width(Fill).wrapping(Wrapping::None),
            coin_content.width(Fill),
            text(side_str)
                .size(12)
                .color(side_color)
                .width(Fill)
                .wrapping(Wrapping::None),
            text(&fill.dir)
                .size(12)
                .width(Fill)
                .wrapping(Wrapping::None),
            text(valid_history_wire_value(&fill.px))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(optional_value_color(
                    parse_history_number(&fill.px),
                    weak_color,
                    invalid_color
                ))
                .width(Fill)
                .wrapping(Wrapping::None),
            text(valid_history_wire_value(&fill.sz))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(optional_value_color(
                    parse_history_number(&fill.sz),
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
        .spacing(4)
        .into()
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
    match fee_token.map(str::trim) {
        Some(token) if !token.is_empty() && !token.eq_ignore_ascii_case("USDC") => {
            format_token_fee(fee, token)
        }
        _ => format!(
            "-{}",
            format_history_display_usd(denomination, Some(fee), 2)
        ),
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
