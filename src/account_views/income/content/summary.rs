use crate::account_analytics::IncomeSnapshot;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::{label_value, label_value_colored, vertical_spacer};
use crate::message::Message;
use iced::widget::{row, text};
use iced::{Element, Theme, color};

pub(super) fn income_earned_total_row(
    data: &IncomeSnapshot,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    row![label_value_colored(
        "Interest Earned (Total)",
        denomination.format_signed_value(data.earned_total, 2),
        if data.earned_total >= 0.0 {
            theme.palette().success
        } else {
            theme.palette().danger
        },
    ),]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

pub(super) fn income_earned_windows_row(
    data: &IncomeSnapshot,
    denomination: &DisplayDenominationContext,
) -> Element<'static, Message> {
    row![
        label_value(
            "Interest 24H",
            denomination.format_signed_value(data.earned_24h, 2)
        ),
        vertical_spacer(),
        label_value(
            "Interest 7D",
            denomination.format_signed_value(data.earned_7d, 2)
        ),
        vertical_spacer(),
        label_value(
            "Interest 30D",
            denomination.format_signed_value(data.earned_30d, 2)
        ),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

pub(super) fn income_interest_note() -> Element<'static, Message> {
    text("24H/7D/30D are realized interest earned over trailing time windows (UTC).")
        .size(10)
        .color(color!(0x7f8ab0))
        .into()
}

pub(super) fn income_carrying_top_row(
    data: &IncomeSnapshot,
    denomination: &DisplayDenominationContext,
) -> Element<'static, Message> {
    row![
        label_value(
            "Projected net / year",
            denomination.format_signed_value(data.net_yearly_projection, 2),
        ),
        vertical_spacer(),
        label_value(
            "Supplied",
            denomination.format_signed_value(data.current_supply_usd, 2),
        ),
        vertical_spacer(),
        label_value(
            "Borrowed",
            denomination.format_signed_value(data.current_borrow_usd, 2),
        ),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

pub(super) fn income_carrying_bottom_row(data: &IncomeSnapshot) -> Element<'static, Message> {
    row![
        label_value("Health", &data.health),
        vertical_spacer(),
        label_value(
            "Health Factor",
            data.health_factor.as_deref().unwrap_or("\u{2014}"),
        ),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}
