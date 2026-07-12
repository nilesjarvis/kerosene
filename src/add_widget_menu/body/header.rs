use super::context::AddWidgetMenuContext;
use crate::message::Message;

use iced::widget::{column, container, text};
use iced::{Element, Fill, Theme};

pub(super) fn target_header(
    _context: &AddWidgetMenuContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let text_color = theme.palette().text;

    container(column![text("Add Widget").size(12).color(text_color)])
        .padding([6, 10])
        .width(Fill)
        .into()
}
