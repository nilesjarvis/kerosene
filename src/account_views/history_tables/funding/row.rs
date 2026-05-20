use crate::account;
use crate::account_views::history::format_history_time_millis;
use crate::account_views::history_tables::numbers::{
    invalid_history_data, parse_history_number, valid_history_wire_value,
};
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{Space, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_funding_history_row<'a>(
        &'a self,
        entry: &'a account::FundingEntry,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let d = &entry.delta;
        let usdc = parse_history_number(&d.usdc);
        let rate = parse_history_number(&d.funding_rate);
        let szi = parse_history_number(&d.szi);
        let weak_color = theme.extended_palette().background.weak.text;
        let invalid_color = theme.palette().warning;
        let amount_color = signed_funding_color(usdc, theme);

        let time_str = format_history_time_millis(entry.time);

        let rate_color = signed_funding_color(rate, theme);

        let denomination = self.display_denomination_context();
        let amount_display = funding_amount_display(&denomination, usdc, self.hide_pnl);

        let mut coin_content = row![];
        if let Some(icon) = helpers::symbol_icon(&d.coin, 14, theme.palette().text) {
            coin_content = coin_content.push(icon).push(Space::new().width(4.0));
        }
        coin_content = coin_content
            .push(text(&d.coin).size(12))
            .align_y(iced::Alignment::Center);

        row![
            text(time_str).size(12).width(Fill),
            coin_content.width(Fill),
            text(funding_rate_display(rate))
                .size(12)
                .color(rate_color)
                .width(Fill),
            text(valid_history_wire_value(&d.szi))
                .size(12)
                .color(history_value_color(szi, weak_color, invalid_color))
                .width(Fill),
            text(amount_display)
                .size(12)
                .color(amount_color)
                .width(Fill),
        ]
        .spacing(4)
        .into()
    }
}

fn signed_funding_color(value: Option<f64>, theme: &Theme) -> Color {
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

fn funding_rate_display(rate: Option<f64>) -> String {
    rate.map(|rate| format!("{:.4}%", rate * 100.0))
        .unwrap_or_else(invalid_history_data)
}

fn funding_amount_display(
    denomination: &crate::denomination::DisplayDenominationContext,
    usdc: Option<f64>,
    hide_pnl: bool,
) -> String {
    if hide_pnl {
        denomination.hidden_mask()
    } else {
        usdc.map(|usdc| denomination.format_signed_value(usdc, 4))
            .unwrap_or_else(invalid_history_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn funding_rate_display_marks_invalid_values() {
        assert_eq!(funding_rate_display(Some(0.00125)), "0.1250%");
        assert_eq!(funding_rate_display(None), "Invalid data");
    }

    #[test]
    fn funding_amount_display_marks_invalid_values() {
        let denomination = crate::denomination::DisplayDenominationContext::default();
        assert_eq!(
            funding_amount_display(&denomination, Some(1.25), false),
            "+$1.2500"
        );
        assert_eq!(
            funding_amount_display(&denomination, Some(-1.25), false),
            "-$1.2500"
        );
        assert_eq!(
            funding_amount_display(&denomination, None, false),
            "Invalid data"
        );
        assert_eq!(funding_amount_display(&denomination, None, true), "$***");
    }
}
