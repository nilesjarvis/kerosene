use crate::message::Message;
use iced::widget::{Space, button, container, pane_grid, text};
use iced::{Element, Fill, Theme};

pub(super) fn pane_drag_ghost_body() -> Element<'static, Message> {
    container(Space::new().width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .into()
}

pub(super) fn pane_close_button(
    pane: pane_grid::Pane,
    pane_count: usize,
    can_close_pane: bool,
) -> button::Button<'static, Message> {
    if pane_count > 1 && can_close_pane {
        button(text("x").size(10).center())
            .on_press(Message::ClosePane(pane))
            .padding([2, 5])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => iced::Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        width: 1.0,
                        color: match status {
                            button::Status::Hovered => theme.palette().danger,
                            _ => iced::Color::TRANSPARENT,
                        },
                        radius: 2.0.into(),
                    },
                    ..Default::default()
                }
            })
    } else {
        button(Space::new().width(10.0).height(10.0)).style(|_theme: &Theme, _status| {
            button::Style {
                background: None,
                ..Default::default()
            }
        })
    }
}
