use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, button, container, row, text, tooltip};
use iced::{Color, Element, Theme};

pub(super) fn tracked_trade_status_dot(color: Color) -> Element<'static, Message> {
    container(Space::new().width(8.0).height(8.0))
        .style(move |_| container_style::Style {
            background: Some(color.into()),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

pub(super) fn tracked_trade_connection_button(
    label: String,
    detail: String,
    status_color: Color,
) -> Element<'static, Message> {
    tooltip(
        button(
            row![tracked_trade_status_dot(status_color), text(label).size(10),]
                .spacing(8)
                .align_y(iced::Alignment::Center),
        )
        .on_press(Message::ReconnectTrackedTrades)
        .padding([2, 6])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.extended_palette().background.weak.text,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }),
        text(detail).size(10),
        tooltip::Position::Top,
    )
    .into()
}

pub(super) fn tracked_trade_settings_button(menu_open: bool) -> Element<'static, Message> {
    tooltip(
        button(text("\u{2699}").size(13).center())
            .on_press(Message::ToggleTrackedTradeSettingsMenu)
            .padding([2, 7])
            .style(move |theme: &Theme, status| {
                let bg = match (menu_open, status) {
                    (_, button::Status::Hovered) => {
                        theme.extended_palette().background.strong.color
                    }
                    (true, _) => theme.extended_palette().background.strong.color,
                    (false, _) => theme.extended_palette().background.weak.color,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: if menu_open {
                        theme.palette().primary
                    } else {
                        theme.palette().text
                    },
                    border: iced::Border {
                        radius: 3.0.into(),
                        width: if menu_open { 1.0 } else { 0.0 },
                        color: Color {
                            a: 0.45,
                            ..theme.palette().primary
                        },
                    },
                    ..Default::default()
                }
            }),
        text("Wallet Tracker settings").size(10),
        tooltip::Position::Top,
    )
    .into()
}

pub(super) fn tracked_trade_settings_dropdown(
    aggregation_enabled: bool,
    alerts_enabled: bool,
) -> Element<'static, Message> {
    let aggregation_btn = tracked_trade_toggle_button(
        if aggregation_enabled {
            "Rows: Orders"
        } else {
            "Rows: Fills"
        },
        aggregation_enabled,
        true,
        Message::ToggleTrackedTradeAggregation,
    );
    let alerts_btn = tracked_trade_toggle_button(
        if alerts_enabled {
            "Alerts: ON"
        } else {
            "Alerts: OFF"
        },
        alerts_enabled,
        false,
        Message::ToggleTrackedTradeAlerts,
    );

    container(
        row![aggregation_btn, alerts_btn]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .wrap()
            .vertical_spacing(6),
    )
    .padding([6, 8])
    .style(|theme: &Theme| container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: Color {
                a: 0.32,
                ..theme.extended_palette().background.strong.color
            },
        },
        ..Default::default()
    })
    .into()
}

pub(super) fn tracked_trade_clear_button() -> Element<'static, Message> {
    button(text("Clear").size(10).center())
        .on_press(Message::ClearTrackedTrades)
        .padding([2, 6])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn tracked_trade_toggle_button(
    label: &'static str,
    enabled: bool,
    primary_when_enabled: bool,
    message: Message,
) -> Element<'static, Message> {
    button(text(label).size(10))
        .on_press(message)
        .padding([2, 6])
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if enabled {
                    if primary_when_enabled {
                        theme.palette().primary
                    } else {
                        theme.palette().success
                    }
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}
