use crate::config;
use crate::message::Message;
use crate::positioning_state::{
    PositioningInfoChangeSortField, PositioningInfoId, PositioningInfoSortField,
};

use iced::alignment::Horizontal;
use iced::widget::{Row, button, text};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

mod trader;

#[cfg(test)]
pub(in crate::market_views::positioning_info) use trader::positioning_trader_action_visibility;
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
            .color(color)
            .width(Fill)
            .align_x(Horizontal::Right),
    );
    if is_active {
        let icon = if sort_direction == config::SortDirection::Ascending {
            "\u{2191}"
        } else {
            "\u{2193}"
        };
        content = content.push(text(icon).size(10).color(color));
    }

    button(content)
        .on_press(Message::PositioningInfoSortChanged(id, field))
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            ..Default::default()
        })
        .padding(0)
        .width(width)
        .into()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn change_sort_header_cell(
    label: impl Into<String>,
    field: PositioningInfoChangeSortField,
    id: PositioningInfoId,
    sort_field: PositioningInfoChangeSortField,
    sort_direction: config::SortDirection,
    width: Length,
    color: Color,
    alignment: Horizontal,
) -> Element<'static, Message> {
    let is_active = sort_field == field;
    let label = label.into();
    let mut content = Row::new().spacing(2).align_y(Alignment::Center).push(
        text(label)
            .size(10)
            .color(color)
            .width(Fill)
            .align_x(alignment),
    );
    if is_active {
        let icon = if sort_direction == config::SortDirection::Ascending {
            "\u{2191}"
        } else {
            "\u{2193}"
        };
        content = content.push(text(icon).size(10).color(color));
    }

    button(content)
        .on_press(Message::PositioningInfoChangeSortChanged(id, field))
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            ..Default::default()
        })
        .padding(0)
        .width(width)
        .into()
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
        .width(width);
    if align_right {
        cell.align_x(Horizontal::Right).into()
    } else {
        cell.into()
    }
}
