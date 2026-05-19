use crate::account::SpotBalance;
use crate::helpers::{self, format_usd};
use crate::message::Message;

use iced::widget::{Space, button, container, row, text};
use iced::{Color, Element, Fill, Theme};

#[cfg(test)]
mod tests;

pub(super) fn balance_row(
    balance: &SpotBalance,
    display_coin: String,
    outcome_sell_coin: Option<String>,
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
    let (total_str, avail_str, hold_str) = balance_amounts(&coin, total, available, hold);
    let entry_str = entry_notional_text(entry_ntl);
    let total_color = balance_number_color(total, theme);
    let available_color = balance_number_color(available, theme);
    let hold_color = balance_number_color(hold, theme);
    let entry_color = balance_number_color(entry_ntl, theme);
    let action_cell: Element<'static, Message> = match (outcome_sell_coin, available) {
        (Some(_), Some(available)) if available.floor() <= 0.0 => text("").size(12).into(),
        (Some(_), None) => text("").size(12).into(),
        (Some(_trade_coin), Some(_)) => {
            button(text("Sell").size(10).center().color(theme.palette().danger))
                .on_press(Message::PrefillOutcomeSell(coin.clone()))
                .padding([1, 6])
                .style(|theme: &Theme, _status| button::Style {
                    background: Some(
                        Color {
                            a: 0.15,
                            ..theme.palette().danger
                        }
                        .into(),
                    ),
                    text_color: theme.palette().danger,
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into()
        }
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
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn balance_amounts(
    coin: &str,
    total: Option<f64>,
    available: Option<f64>,
    hold: Option<f64>,
) -> (String, String, String) {
    (
        balance_amount(coin, total),
        balance_amount(coin, available),
        balance_amount(coin, hold),
    )
}

fn balance_amount(coin: &str, value: Option<f64>) -> String {
    match value {
        Some(value) if matches!(coin, "USDC" | "USDH") => format_usd(&format!("{value:.2}")),
        Some(value) if coin.starts_with('+') => format!("{:.0}", value.floor()),
        Some(value) => format!("{value:.6}"),
        None => "Invalid data".to_string(),
    }
}

fn entry_notional_text(entry_ntl: Option<f64>) -> String {
    match entry_ntl {
        Some(entry_ntl) if entry_ntl.abs() > 0.0 => format_usd(&format!("{entry_ntl:.2}")),
        Some(_) => "\u{2014}".to_string(),
        None => "Invalid data".to_string(),
    }
}

fn balance_number_color(value: Option<f64>, theme: &Theme) -> Color {
    if value.is_some() {
        theme.extended_palette().background.weak.text
    } else {
        theme.palette().warning
    }
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
