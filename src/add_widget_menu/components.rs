use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{button, container, row, text};
use iced::{Color, Element, Fill, Theme, color};

// ---------------------------------------------------------------------------
// Add-widget menu components
// ---------------------------------------------------------------------------

fn menu_tag(label: &'static str, theme: &Theme) -> Element<'static, Message> {
    let color = tag_color(label, theme);
    container(text(label).size(9).color(color))
        .padding([1, 5])
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(Color { a: 0.12, ..color }.into()),
            border: iced::Border {
                radius: 3.0.into(),
                width: 1.0,
                color: Color { a: 0.45, ..color },
            },
            ..Default::default()
        })
        .into()
}

fn tag_color(label: &str, theme: &Theme) -> Color {
    match label {
        "Open" => theme.palette().success,
        "Window" => theme.palette().primary,
        "Requires PM" => color!(0xffb86c),
        _ => theme.extended_palette().background.weak.text,
    }
}

pub(super) fn menu_item(
    label: &'static str,
    tag: &'static str,
    message: Option<Message>,
    enabled: bool,
    theme: &Theme,
) -> Element<'static, Message> {
    let press_message = if enabled { message } else { None };

    button(
        row![text(label).size(11).width(Fill), menu_tag(tag, theme),]
            .spacing(8)
            .align_y(iced::Alignment::Center),
    )
    .on_press_maybe(press_message)
    .padding([6, 8])
    .width(Fill)
    .style(move |theme: &Theme, status| {
        let bg = match status {
            button::Status::Hovered if enabled => theme.extended_palette().background.strong.color,
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(bg.into()),
            text_color: if enabled {
                theme.palette().text
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

pub(super) fn section_label(label: &'static str, theme: &Theme) -> Element<'static, Message> {
    let color = theme.extended_palette().background.weak.text;
    container(text(label).size(10).color(color).width(Fill))
        .padding([6, 10])
        .width(Fill)
        .into()
}
