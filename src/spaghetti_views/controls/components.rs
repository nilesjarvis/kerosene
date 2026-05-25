use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartId;
use iced::widget::{Row, button, container, pick_list, rule, text};
use iced::{Color, Element, Fill, Theme, color};

pub(super) fn spaghetti_controls_strip<'a>(content: Row<'a, Message>) -> Element<'a, Message> {
    container(content.width(Fill).wrap().vertical_spacing(0))
        .width(Fill)
        .style(|theme: &Theme| {
            let background = Color {
                a: 0.04,
                ..theme.extended_palette().background.weak.color
            };
            container::Style {
                background: Some(background.into()),
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn spaghetti_controls_button(
    label: &'static str,
    active: bool,
    msg: Message,
) -> Element<'static, Message> {
    button(text(label).size(11).center())
        .on_press(msg)
        .padding([3, 8])
        .style(move |theme: &Theme, status| spaghetti_controls_button_style(theme, status, active))
        .into()
}

pub(super) fn reload_button(id: SpaghettiChartId) -> Element<'static, Message> {
    button(text("\u{27F3}").size(12))
        .on_press(Message::SpaghettiReload(id))
        .padding([3, 8])
        .style(|theme: &Theme, status| spaghetti_controls_button_style(theme, status, false))
        .into()
}

pub(super) fn reset_view_button(id: SpaghettiChartId) -> Element<'static, Message> {
    spaghetti_controls_button("Reset View", false, Message::SpaghettiResetView(id))
}

pub(super) fn style_button(id: SpaghettiChartId, open: bool) -> Element<'static, Message> {
    spaghetti_controls_button("STYLE", open, Message::ToggleSpaghettiStyleMenu(id))
}

pub(super) fn spaghetti_controls_separator() -> Element<'static, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.12,
            ..theme.extended_palette().background.weak.text
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .height(14)
    .width(1)
    .into()
}

pub(super) fn spaghetti_controls_status_label(label: String) -> Element<'static, Message> {
    container(text(label).size(10).color(color!(0x8e9cc2)))
        .padding([3, 8])
        .into()
}

fn spaghetti_controls_button_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let background = if active {
        Color {
            a: 0.10,
            ..theme.palette().primary
        }
    } else {
        match status {
            button::Status::Hovered => Color {
                a: 0.55,
                ..theme.extended_palette().background.strong.color
            },
            _ => Color::TRANSPARENT,
        }
    };

    button::Style {
        background: Some(background.into()),
        text_color: if active {
            theme.palette().primary
        } else {
            theme.extended_palette().background.weak.text
        },
        border: iced::Border {
            radius: 0.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn spaghetti_controls_pick_list_style(
    theme: &Theme,
    status: pick_list::Status,
) -> pick_list::Style {
    let background = match status {
        pick_list::Status::Hovered | pick_list::Status::Opened { .. } => Color {
            a: 0.55,
            ..theme.extended_palette().background.strong.color
        },
        pick_list::Status::Active => Color::TRANSPARENT,
    };

    pick_list::Style {
        text_color: theme.extended_palette().background.weak.text,
        placeholder_color: theme.extended_palette().background.weak.text,
        handle_color: theme.extended_palette().background.weak.text,
        background: background.into(),
        border: iced::Border {
            radius: 0.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
    }
}
