use crate::account::SpotBalance;
use crate::denomination::DisplayDenominationContext;
use crate::message::Message;
use crate::wallet_views::numbers::{
    format_wallet_display_amount, format_wallet_display_usd, invalid_wallet_data,
    parse_wallet_number,
};

use iced::widget::{Row, row, text};
use iced::{Color, Element, Theme};

#[cfg(test)]
mod tests;

pub(super) fn wallet_spot_header() -> Row<'static, Message> {
    row![
        text("Asset").size(10).width(90),
        text("Total").size(10).width(110),
        text("Hold").size(10).width(110),
        text("Available").size(10).width(110),
        text("Entry Ntl").size(10).width(110),
        text("Supplied").size(10).width(110),
    ]
    .spacing(8)
}

pub(super) fn wallet_spot_row(
    balance: &SpotBalance,
    display_coin: String,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let total = parse_wallet_number(&balance.total);
    let hold = parse_wallet_number(&balance.hold);
    let available = total.zip(hold).map(|(total, hold)| total - hold);
    let entry_ntl = parse_wallet_number(&balance.entry_ntl);
    let supplied = wallet_supplied_amount(
        denomination,
        balance.supplied.as_deref(),
        balance.coin == "USDC",
    );
    let is_usdc = balance.coin == "USDC";
    let weak_color = theme.extended_palette().background.weak.text;
    let invalid_color = theme.palette().warning;

    row![
        text(display_coin)
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(if is_usdc {
                theme.palette().text
            } else {
                theme.palette().success
            })
            .width(90),
        text(format_wallet_display_amount(denomination, total, is_usdc))
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(wallet_spot_value_color(total, weak_color, invalid_color))
            .width(110),
        text(format_wallet_display_amount(denomination, hold, is_usdc))
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(wallet_spot_value_color(hold, weak_color, invalid_color))
            .width(110),
        text(format_wallet_display_amount(
            denomination,
            available,
            is_usdc
        ))
        .size(11)
        .font(crate::app_fonts::monospace_font())
        .color(wallet_spot_value_color(
            available,
            weak_color,
            invalid_color
        ))
        .width(110),
        text(wallet_entry_notional(denomination, entry_ntl))
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(wallet_spot_value_color(
                entry_ntl,
                weak_color,
                invalid_color
            ))
            .width(110),
        text(supplied.clone())
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(if supplied == invalid_wallet_data() {
                invalid_color
            } else {
                weak_color
            })
            .width(110),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn wallet_entry_notional(
    denomination: &DisplayDenominationContext,
    entry_ntl: Option<f64>,
) -> String {
    match entry_ntl {
        Some(entry_ntl) if entry_ntl.abs() > 0.0 => {
            format_wallet_display_usd(denomination, Some(entry_ntl), 2)
        }
        Some(_) => "-".to_string(),
        None => invalid_wallet_data(),
    }
}

fn wallet_supplied_amount(
    denomination: &DisplayDenominationContext,
    value: Option<&str>,
    is_usdc: bool,
) -> String {
    match value {
        Some(value) => {
            format_wallet_display_amount(denomination, parse_wallet_number(value), is_usdc)
        }
        None => "-".to_string(),
    }
}

fn wallet_spot_value_color(
    value: Option<f64>,
    default_color: Color,
    invalid_color: Color,
) -> Color {
    if value.is_some() {
        default_color
    } else {
        invalid_color
    }
}
