use super::context::AddWidgetMenuContext;
use crate::message::Message;
use crate::pane_management::AddWidgetPlacement;

use super::super::components::placement_button;
use iced::widget::{column, container, row, text};
use iced::{Element, Fill, Theme};

pub(super) fn target_header(
    context: &AddWidgetMenuContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let text_color = theme.palette().text;
    let weak_text_color = theme.extended_palette().background.weak.text;

    container(
        column![
            text("Add Widget").size(12).color(text_color),
            text(format!("Target: {}", context.target_title))
                .size(10)
                .color(weak_text_color),
        ]
        .spacing(2),
    )
    .padding([6, 10])
    .width(Fill)
    .into()
}

pub(super) fn placement_controls(placement: AddWidgetPlacement) -> Element<'static, Message> {
    container(
        row![
            placement_button(
                placement == AddWidgetPlacement::Below,
                AddWidgetPlacement::Below,
                "Below"
            ),
            placement_button(
                placement == AddWidgetPlacement::Right,
                AddWidgetPlacement::Right,
                "Right"
            ),
        ]
        .spacing(6),
    )
    .padding([0, 6])
    .into()
}
