use crate::account;
use crate::account_views::history::format_history_time_millis;
use crate::account_views::history_tables::numbers::{
    invalid_history_data, parse_history_number, valid_history_wire_value,
};
use crate::account_views::history_tables::style::history_signed_value_color;
use crate::app_state::TradingTerminal;
use crate::helpers::{self, optional_value_color};
use crate::message::Message;
use iced::widget::{Space, row, text};
use iced::{Element, Fill, Theme};

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
        let amount_color = history_signed_value_color(usdc, theme);

        let time_str = format_history_time_millis(entry.time);

        let rate_color = history_signed_value_color(rate, theme);

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
                .color(optional_value_color(szi, weak_color, invalid_color))
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
#[path = "row/tests.rs"]
mod tests;
