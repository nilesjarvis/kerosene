use crate::account;
use crate::account_views::history::format_history_time_millis;
use crate::account_views::history_tables::numbers::{
    format_history_display_usd, invalid_history_data, parse_history_number,
    valid_history_wire_value,
};
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{Row, Space, row, text};
use iced::{Color, Element, Fill, Theme};

pub(super) fn view_trade_history_header(theme: &Theme) -> Row<'static, Message> {
    let header_txt = |s: &'static str| {
        text(s)
            .size(11)
            .color(theme.extended_palette().background.weak.text)
            .width(Fill)
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
        let pnl_color = signed_history_color(pnl, theme);
        let fee_color = history_value_color(fee, weak_color, invalid_color);
        let denomination = self.display_denomination_context();

        let pnl_display = if self.hide_pnl {
            denomination.hidden_mask()
        } else {
            format_history_display_usd(&denomination, pnl, 2)
        };
        let fee_display = history_fee_display(&denomination, fee, self.hide_pnl);

        let mut coin_content = row![];
        if let Some(icon) = helpers::symbol_icon(&fill.coin, 14, theme.palette().text) {
            coin_content = coin_content.push(icon).push(Space::new().width(4.0));
        }
        coin_content = coin_content
            .push(text(&fill.coin).size(12))
            .align_y(iced::Alignment::Center);

        row![
            text(time_str).size(12).width(Fill),
            coin_content.width(Fill),
            text(side_str).size(12).color(side_color).width(Fill),
            text(&fill.dir).size(12).width(Fill),
            text(valid_history_wire_value(&fill.px))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(history_value_color(
                    parse_history_number(&fill.px),
                    weak_color,
                    invalid_color
                ))
                .width(Fill),
            text(valid_history_wire_value(&fill.sz))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(history_value_color(
                    parse_history_number(&fill.sz),
                    weak_color,
                    invalid_color
                ))
                .width(Fill),
            text(pnl_display).size(12).color(pnl_color).width(Fill),
            text(fee_display).size(12).color(fee_color).width(Fill),
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

fn signed_history_color(value: Option<f64>, theme: &Theme) -> Color {
    match value {
        Some(value) if value >= 0.0 => theme.palette().success,
        Some(_) => theme.palette().danger,
        None => theme.palette().warning,
    }
}

fn history_value_color(value: Option<f64>, default_color: Color, invalid_color: Color) -> Color {
    if value.is_some() {
        default_color
    } else {
        invalid_color
    }
}

fn history_fee_display(
    denomination: &crate::denomination::DisplayDenominationContext,
    fee: Option<f64>,
    hide_pnl: bool,
) -> String {
    if hide_pnl {
        denomination.hidden_mask()
    } else {
        fee.map(|fee| {
            format!(
                "-{}",
                format_history_display_usd(denomination, Some(fee), 2)
            )
        })
        .unwrap_or_else(invalid_history_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_fee_display_marks_invalid_values() {
        let denomination = crate::denomination::DisplayDenominationContext::default();
        assert_eq!(
            history_fee_display(&denomination, Some(1.25), false),
            "-$1.25"
        );
        assert_eq!(
            history_fee_display(&denomination, None, false),
            "Invalid data"
        );
        assert_eq!(history_fee_display(&denomination, None, true), "$***");
    }
}
