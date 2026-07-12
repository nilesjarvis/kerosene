use crate::message::Message;
use crate::pane_management::AddWidgetPlacement;
use iced::widget::{
    Space, button, column, container, mouse_area, pane_grid, responsive, row, text,
};
use iced::{Element, Fill, Point, Size, Theme};

pub(super) fn pane_drag_ghost_body() -> Element<'static, Message> {
    container(Space::new().width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .into()
}

pub(super) fn widget_placement_overlay(
    pane: pane_grid::Pane,
    label: &'static str,
    hovered_placement: Option<AddWidgetPlacement>,
) -> Element<'static, Message> {
    responsive(move |size| widget_placement_overlay_sized(pane, label, hovered_placement, size))
        .width(Fill)
        .height(Fill)
        .into()
}

fn widget_placement_overlay_sized(
    pane: pane_grid::Pane,
    label: &'static str,
    hovered_placement: Option<AddWidgetPlacement>,
    size: Size,
) -> Element<'static, Message> {
    let Some(placement) = hovered_placement else {
        return mouse_area(container(Space::new().width(Fill).height(Fill)))
            .on_move(move |position| {
                Message::WidgetPlacementHovered(pane, placement_for_cursor(position, size))
            })
            .on_exit(Message::WidgetPlacementExited(pane))
            .on_press(Message::PlaceWidget(pane, AddWidgetPlacement::Below))
            .interaction(iced::mouse::Interaction::Crosshair)
            .into();
    };

    let preview = container(
        column![text("+").size(20), text(label).size(11)]
            .spacing(3)
            .align_x(iced::Alignment::Center),
    )
    .width(Fill)
    .height(Fill)
    .center_x(Fill)
    .center_y(Fill)
    .style(move |theme: &Theme| {
        let mut background = theme.palette().primary;
        background.a = 0.20;
        let mut border = theme.palette().primary;
        border.a = 0.95;

        iced::widget::container::Style {
            background: Some(background.into()),
            text_color: Some(theme.palette().primary),
            border: iced::Border {
                width: 2.0,
                color: border,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    });

    let preview: Element<'static, Message> = match placement {
        AddWidgetPlacement::Left => row![preview, Space::new().width(Fill)]
            .width(Fill)
            .height(Fill)
            .into(),
        AddWidgetPlacement::Below => column![Space::new().height(Fill), preview]
            .width(Fill)
            .height(Fill)
            .into(),
        AddWidgetPlacement::Right => row![Space::new().width(Fill), preview]
            .width(Fill)
            .height(Fill)
            .into(),
    };

    let overlay = container(preview)
        .width(Fill)
        .height(Fill)
        .style(|theme: &Theme| {
            let mut dim = theme.palette().background;
            dim.a = 0.52;
            iced::widget::container::Style {
                background: Some(dim.into()),
                ..Default::default()
            }
        });

    mouse_area(overlay)
        .on_move(move |position| {
            Message::WidgetPlacementHovered(pane, placement_for_cursor(position, size))
        })
        .on_exit(Message::WidgetPlacementExited(pane))
        .on_press(Message::PlaceWidget(pane, placement))
        .interaction(iced::mouse::Interaction::Crosshair)
        .into()
}

fn placement_for_cursor(position: Point, size: Size) -> AddWidgetPlacement {
    if !size.width.is_finite()
        || !size.height.is_finite()
        || size.width <= 0.0
        || size.height <= 0.0
    {
        return AddWidgetPlacement::Below;
    }

    fn progress(value: f32, extent: f32) -> f32 {
        if value.is_finite() && extent.is_finite() && extent > 0.0 {
            (value / extent).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    let horizontal = progress(position.x, size.width);
    let left_distance = horizontal;
    let right_distance = 1.0 - horizontal;
    let bottom_distance = 1.0 - progress(position.y, size.height);

    if bottom_distance <= left_distance.min(right_distance) {
        AddWidgetPlacement::Below
    } else if left_distance < right_distance {
        AddWidgetPlacement::Left
    } else {
        AddWidgetPlacement::Right
    }
}

#[cfg(test)]
mod placement_tests {
    use super::*;

    #[test]
    fn cursor_position_selects_the_nearest_supported_edge() {
        let size = Size::new(800.0, 400.0);

        assert_eq!(
            placement_for_cursor(Point::new(50.0, 50.0), size),
            AddWidgetPlacement::Left
        );
        assert_eq!(
            placement_for_cursor(Point::new(700.0, 50.0), size),
            AddWidgetPlacement::Right
        );
        assert_eq!(
            placement_for_cursor(Point::new(100.0, 350.0), size),
            AddWidgetPlacement::Below
        );
    }

    #[test]
    fn cursor_placement_handles_degenerate_bounds() {
        assert_eq!(
            placement_for_cursor(Point::new(f32::NAN, 1.0), Size::ZERO),
            AddWidgetPlacement::Below
        );
    }
}

pub(super) fn pane_refresh_button() -> button::Button<'static, Message> {
    button(
        text("\u{21bb}")
            .size(11)
            .center()
            .font(crate::app_fonts::monospace_font()),
    )
    .on_press(Message::RefreshPortfolio)
    .padding([2, 5])
    .style(|theme: &Theme, status| {
        let bg = match status {
            button::Status::Hovered => theme.extended_palette().background.strong.color,
            _ => iced::Color::TRANSPARENT,
        };
        button::Style {
            background: Some(bg.into()),
            text_color: match status {
                button::Status::Hovered => theme.palette().primary,
                _ => iced::Color {
                    a: 0.55,
                    ..theme.palette().text
                },
            },
            border: iced::Border {
                radius: 2.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
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
