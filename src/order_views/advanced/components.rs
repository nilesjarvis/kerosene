use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{button, container, text};
use iced::{Color, Element, Theme};

mod spinning;

pub(super) use spinning::spinning_gear;

// ---------------------------------------------------------------------------
// Shared Row Components
// ---------------------------------------------------------------------------

pub(super) fn stop_button(chase_id: u64) -> Element<'static, Message> {
    button(
        text("Stop")
            .size(10)
            .center()
            .width(iced::Length::Fixed(44.0)),
    )
    .on_press(Message::StopChaseById(chase_id))
    .padding([3, 6])
    .style(|theme: &Theme, status| danger_button_style(theme, status))
    .into()
}

pub(super) fn stop_twap_button(twap_id: u64) -> Element<'static, Message> {
    stop_like_button("Stop", Message::StopTwap(twap_id))
}

pub(super) fn details_button(twap_id: u64) -> Element<'static, Message> {
    button(
        text("Info")
            .size(10)
            .center()
            .width(iced::Length::Fixed(36.0)),
    )
    .on_press(Message::OpenTwapDetails(twap_id))
    .padding([3, 6])
    .style(|theme: &Theme, status| info_button_style(theme, status))
    .into()
}

pub(super) fn history_info_button(entry_id: String) -> Element<'static, Message> {
    button(
        text("Info")
            .size(10)
            .center()
            .width(iced::Length::Fixed(36.0)),
    )
    .on_press(Message::OpenAdvancedOrderHistory(entry_id.into()))
    .padding([3, 6])
    .style(|theme: &Theme, status| info_button_style(theme, status))
    .into()
}

pub(super) fn stop_all_button() -> Element<'static, Message> {
    button(text("Stop All").size(10).center())
        .on_press(Message::StopAllAdvancedOrders)
        .padding([3, 8])
        .style(|theme: &Theme, status| danger_button_style(theme, status))
        .into()
}

pub(super) fn badge(label: &'static str) -> Element<'static, Message> {
    container(text(label).size(9).center())
        .padding([2, 3])
        .style(|theme: &Theme| container_style::Style {
            background: Some(theme.extended_palette().background.strong.color.into()),
            text_color: Some(theme.extended_palette().background.strong.text),
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

pub(super) fn row_container_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}

pub(super) fn history_row_container_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.weak.color,
        },
        ..Default::default()
    }
}

fn stop_like_button(label: &'static str, message: Message) -> Element<'static, Message> {
    button(
        text(label)
            .size(10)
            .center()
            .width(iced::Length::Fixed(44.0)),
    )
    .on_press(message)
    .padding([3, 6])
    .style(|theme: &Theme, status| danger_button_style(theme, status))
    .into()
}

fn danger_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let bg = button_background(theme, status);
    let danger = theme.palette().danger;
    button::Style {
        background: Some(bg.into()),
        text_color: danger,
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: Color { a: 0.45, ..danger },
        },
        ..Default::default()
    }
}

fn info_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let bg = button_background(theme, status);
    let primary = theme.palette().primary;
    button::Style {
        background: Some(bg.into()),
        text_color: primary,
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: Color { a: 0.45, ..primary },
        },
        ..Default::default()
    }
}

fn button_background(theme: &Theme, status: button::Status) -> Color {
    match status {
        button::Status::Hovered => theme.extended_palette().background.strong.color,
        _ => theme.extended_palette().background.weak.color,
    }
}
