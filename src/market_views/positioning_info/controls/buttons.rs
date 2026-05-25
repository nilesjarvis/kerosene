use crate::message::Message;
use crate::positioning_state::{PositioningInfoId, PositioningInfoPage};

use iced::widget::{button, text};
use iced::{Color, Element, Theme};

// ---------------------------------------------------------------------------
// Positioning Control Buttons
// ---------------------------------------------------------------------------

pub(in crate::market_views::positioning_info) fn positioning_control_button(
    label: &'static str,
    active: bool,
    msg: Message,
) -> Element<'static, Message> {
    button(text(label).size(10).center())
        .on_press(msg)
        .padding([2, 7])
        .style(move |theme: &Theme, status| {
            let bg = if active {
                theme.extended_palette().background.strong.color
            } else {
                match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
                }
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if active {
                    theme.palette().text
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 2.0.into(),
                    width: if active { 1.0 } else { 0.0 },
                    color: if active {
                        Color {
                            a: 0.4,
                            ..theme.palette().primary
                        }
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
        .into()
}

pub(in crate::market_views::positioning_info) fn positioning_clear_filters_button(
    id: PositioningInfoId,
    active: bool,
) -> Element<'static, Message> {
    let mut clear_button = button(text("Clear filters").size(10).center())
        .padding([2, 7])
        .style(move |theme: &Theme, status| {
            let text_color = if active {
                theme.extended_palette().primary.base.color
            } else {
                theme.extended_palette().background.weak.text
            };
            let bg = match status {
                button::Status::Hovered if active => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color {
                    a: if active { 1.0 } else { 0.55 },
                    ..text_color
                },
                border: iced::Border {
                    radius: 2.0.into(),
                    width: 1.0,
                    color: Color {
                        a: if active { 0.35 } else { 0.18 },
                        ..text_color
                    },
                },
                ..Default::default()
            }
        });
    if active {
        clear_button = clear_button.on_press(Message::ClearPositioningInfoFilters(id));
    }
    clear_button.into()
}

pub(in crate::market_views::positioning_info) fn positioning_navigation_button(
    id: PositioningInfoId,
    page: PositioningInfoPage,
    active: bool,
) -> Element<'static, Message> {
    button(text(page.label()).size(11).center())
        .on_press(Message::PositioningInfoPageChanged(id, page))
        .padding([3, 9])
        .style(move |theme: &Theme, status| {
            let bg = if active {
                theme.extended_palette().background.strong.color
            } else {
                match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
                }
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if active {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 3.0.into(),
                    width: if active { 1.0 } else { 0.0 },
                    color: if active {
                        Color {
                            a: 0.35,
                            ..theme.palette().primary
                        }
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
        .into()
}
