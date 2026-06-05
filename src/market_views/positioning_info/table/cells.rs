use crate::config;
use crate::message::Message;
use crate::positioning_state::{PositioningInfoId, PositioningInfoSortField};

use iced::alignment::Horizontal;
use iced::widget::text::Wrapping;
use iced::widget::{Row, button, text};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

mod trader;

#[cfg(test)]
pub(in crate::market_views::positioning_info) use trader::positioning_trader_actions_enabled;
pub(super) use trader::positioning_trader_cell;

pub(super) fn header_cell(
    label: &'static str,
    width: Length,
    color: Color,
) -> Element<'static, Message> {
    header_cell_aligned(label, width, color, Horizontal::Right)
}

pub(super) fn header_cell_aligned(
    label: &'static str,
    width: Length,
    color: Color,
    alignment: Horizontal,
) -> Element<'static, Message> {
    text(label)
        .size(10)
        .color(color)
        .width(width)
        .align_x(alignment)
        .wrapping(Wrapping::None)
        .into()
}

pub(super) fn sort_header_cell(
    label: &'static str,
    field: PositioningInfoSortField,
    id: PositioningInfoId,
    sort_field: PositioningInfoSortField,
    sort_direction: config::SortDirection,
    width: Length,
    color: Color,
) -> Element<'static, Message> {
    let is_active = sort_field == field;
    let mut content = Row::new().spacing(2).align_y(Alignment::Center).push(
        text(label)
            .size(10)
            .width(Fill)
            .align_x(Horizontal::Right)
            .wrapping(Wrapping::None),
    );
    content = content.push(sort_arrow(is_active, sort_direction, color));

    button(content)
        .on_press(Message::PositioningInfoSortChanged(id, field))
        .style(sort_header_button_style)
        .padding([1, 2])
        .width(width)
        .into()
}

fn sort_arrow(
    is_active: bool,
    sort_direction: config::SortDirection,
    color: Color,
) -> Element<'static, Message> {
    let icon = if !is_active {
        // Faint neutral marker that signals the column is sortable.
        "\u{2195}"
    } else if sort_direction == config::SortDirection::Ascending {
        "\u{2191}"
    } else {
        "\u{2193}"
    };
    let color = Color {
        a: if is_active { color.a } else { color.a * 0.4 },
        ..color
    };
    text(icon).size(9).color(color).into()
}

fn sort_header_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: if hovered {
            Some(theme.extended_palette().background.weak.color.into())
        } else {
            None
        },
        text_color: if hovered {
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
}

pub(super) fn value_cell(
    value: impl ToString,
    width: Length,
    color: Color,
    align_right: bool,
) -> Element<'static, Message> {
    let cell = text(value.to_string())
        .size(11)
        .font(crate::app_fonts::monospace_font())
        .color(color)
        .width(width)
        .wrapping(Wrapping::None);
    if align_right {
        cell.align_x(Horizontal::Right).into()
    } else {
        cell.into()
    }
}
