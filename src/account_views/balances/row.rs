use crate::account::SpotBalance;
use crate::account_views::invalid_account_data;
use crate::account_views::style::compact_action_button;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::{self, optional_value_color, parse_finite_number};
use crate::message::Message;

use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Fill, Theme};

#[cfg(test)]
mod tests;

pub(super) fn balance_row(
    balance: &SpotBalance,
    display_coin: String,
    outcome_sell_coin: Option<String>,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let total = parse_balance_number(&balance.total);
    let hold = parse_balance_number(&balance.hold);
    let available = total.zip(hold).map(|(total, hold)| total - hold);
    let entry_ntl = parse_balance_number(&balance.entry_ntl);
    let coin = balance.coin.clone();

    let coin_color = if coin == "USDC" {
        Color::WHITE
    } else {
        theme.palette().success
    };
    let (total_str, avail_str, hold_str) =
        balance_amounts(&coin, total, available, hold, denomination);
    let entry_str = entry_notional_text(entry_ntl, denomination);
    let total_color = balance_number_color(total, theme);
    let available_color = balance_number_color(available, theme);
    let hold_color = balance_number_color(hold, theme);
    let entry_color = balance_number_color(entry_ntl, theme);
    let action_cell: Element<'static, Message> = match (outcome_sell_coin, available) {
        (Some(_), Some(available)) if available.floor() <= 0.0 => text("").size(12).into(),
        (Some(_), None) => text("").size(12).into(),
        (Some(_trade_coin), Some(_)) => compact_action_button(
            "Sell",
            theme.palette().danger,
            Message::PrefillOutcomeSell(coin.clone()),
        ),
        (None, _) => text("").size(12).into(),
    };

    row![
        balance_coin_cell(display_coin, coin_color).width(Fill),
        text(total_str).size(12).color(total_color).width(Fill),
        text(hold_str).size(12).color(hold_color).width(Fill),
        text(avail_str).size(12).color(available_color).width(Fill),
        text(entry_str).size(12).color(entry_color).width(Fill),
        container(action_cell).width(60),
    ]
    .spacing(4)
    .into()
}

pub(super) fn balance_has_visible_total(balance: &SpotBalance) -> bool {
    parse_balance_number(&balance.total)
        .map(|total| total.abs() > 0.0)
        .unwrap_or(true)
}

fn parse_balance_number(raw: &str) -> Option<f64> {
    parse_finite_number(raw)
}

fn balance_amounts(
    coin: &str,
    total: Option<f64>,
    available: Option<f64>,
    hold: Option<f64>,
    denomination: &DisplayDenominationContext,
) -> (String, String, String) {
    (
        balance_amount(coin, total, denomination),
        balance_amount(coin, available, denomination),
        balance_amount(coin, hold, denomination),
    )
}

fn balance_amount(
    coin: &str,
    value: Option<f64>,
    denomination: &DisplayDenominationContext,
) -> String {
    match value {
        Some(value) if matches!(coin, "USDC" | "USDH") => denomination.format_value(value, 2),
        Some(value) if coin.starts_with('+') => format!("{:.0}", value.floor()),
        Some(value) => format!("{value:.6}"),
        None => invalid_account_data(),
    }
}

fn entry_notional_text(
    entry_ntl: Option<f64>,
    denomination: &DisplayDenominationContext,
) -> String {
    match entry_ntl {
        Some(entry_ntl) if entry_ntl.abs() > 0.0 => denomination.format_value(entry_ntl, 2),
        Some(_) => "\u{2014}".to_string(),
        None => invalid_account_data(),
    }
}

fn balance_number_color(value: Option<f64>, theme: &Theme) -> Color {
    optional_value_color(
        value,
        theme.extended_palette().background.weak.text,
        theme.palette().warning,
    )
}

fn balance_coin_cell(coin: String, coin_color: Color) -> iced::widget::Row<'static, Message> {
    let mut coin_content = row![];
    if let Some(icon) = helpers::symbol_icon(&coin, 14, coin_color) {
        coin_content = coin_content.push(icon).push(Space::new().width(4.0));
    }

    coin_content
        .push(text(coin).size(12).color(coin_color))
        .align_y(iced::Alignment::Center)
}
