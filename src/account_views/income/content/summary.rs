use crate::account_analytics::IncomeSnapshot;
use crate::account_metrics::format_signed_usd_value;
use crate::helpers::{label_value, label_value_colored, vertical_spacer};
use crate::message::Message;
use iced::widget::{row, text};
use iced::{Element, Theme, color};

pub(super) fn income_earned_total_row(
    data: &IncomeSnapshot,
    theme: &Theme,
) -> Element<'static, Message> {
    row![label_value_colored(
        "Interest Earned (Total)",
        format_signed_usd_value(data.earned_total),
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

pub(super) fn income_earned_windows_row(data: &IncomeSnapshot) -> Element<'static, Message> {
    row![
        label_value("Interest 24H", format_signed_usd_value(data.earned_24h)),
        vertical_spacer(),
        label_value("Interest 7D", format_signed_usd_value(data.earned_7d)),
        vertical_spacer(),
        label_value("Interest 30D", format_signed_usd_value(data.earned_30d)),
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

pub(super) fn income_carrying_top_row(data: &IncomeSnapshot) -> Element<'static, Message> {
    row![
        label_value(
            "Projected net / year",
            format_signed_usd_value(data.net_yearly_projection),
        ),
        vertical_spacer(),
        label_value("Supplied", format_signed_usd_value(data.current_supply_usd)),
        vertical_spacer(),
        label_value("Borrowed", format_signed_usd_value(data.current_borrow_usd)),
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
