use super::TITLE_BAR_HEIGHT;
use crate::message::Message;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{button, container, svg, text, tooltip};
use iced::{Element, Fill, Length};

#[cfg(target_os = "linux")]
const WINDOW_BUTTON_WIDTH: f32 = 42.0;
const CHROME_TOGGLE_BUTTON_WIDTH: f32 = 34.0;
#[cfg(target_os = "linux")]
const WINDOW_BUTTON_ICON_SIZE: f32 = 12.0;
const CHROME_TOGGLE_ICON_SIZE: f32 = 13.0;

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowButtonKind {
    Default,
    Close,
}

pub(super) fn chrome_toggle_button(
    icon_svg: &'static [u8],
    label: &'static str,
    enabled: bool,
    message: Message,
) -> Element<'static, Message> {
    let icon: Element<'static, Message> = svg(SvgHandle::from_memory(icon_svg))
        .width(Length::Fixed(CHROME_TOGGLE_ICON_SIZE))
        .height(Length::Fixed(CHROME_TOGGLE_ICON_SIZE))
        .style(move |theme: &iced::Theme, _status| svg::Style {
            color: Some(chrome_toggle_icon_color(theme, enabled)),
        })
        .into();

    let control = button(container(icon).center_x(Fill).center_y(Fill))
        .on_press(message)
        .width(Length::Fixed(CHROME_TOGGLE_BUTTON_WIDTH))
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .padding(0)
        .style(move |theme: &iced::Theme, status| {
            let background = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    Some(theme.extended_palette().background.weak.color.into())
                }
                _ => None,
            };

            button::Style {
                background,
                text_color: chrome_toggle_icon_color(theme, enabled),
                border: iced::Border {
                    radius: 0.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    tooltip(
        control,
        text(label)
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Bottom,
    )
    .into()
}

fn chrome_toggle_icon_color(theme: &iced::Theme, enabled: bool) -> iced::Color {
    if enabled {
        theme.palette().success
    } else {
        theme.extended_palette().background.weak.text
    }
}

#[cfg(target_os = "linux")]
pub(super) fn window_chrome_button(
    icon_svg: &'static [u8],
    label: &'static str,
    message: Message,
    kind: WindowButtonKind,
) -> Element<'static, Message> {
    let icon: Element<'static, Message> = svg(SvgHandle::from_memory(icon_svg))
        .width(Length::Fixed(WINDOW_BUTTON_ICON_SIZE))
        .height(Length::Fixed(WINDOW_BUTTON_ICON_SIZE))
        .style(|theme: &iced::Theme, _status| svg::Style {
            color: Some(theme.palette().text),
        })
        .into();

    let control = button(container(icon).center_x(Fill).center_y(Fill))
        .on_press(message)
        .width(Length::Fixed(WINDOW_BUTTON_WIDTH))
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .padding(0)
        .style(move |theme: &iced::Theme, status| {
            let palette = theme.extended_palette();
            let background = match (kind, status) {
                (WindowButtonKind::Close, button::Status::Hovered | button::Status::Pressed) => {
                    Some(palette.danger.strong.color.into())
                }
                (_, button::Status::Hovered | button::Status::Pressed) => {
                    Some(palette.background.weak.color.into())
                }
                _ => None,
            };

            button::Style {
                background,
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 0.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    tooltip(
        control,
        text(label)
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Bottom,
    )
    .into()
}
