use super::super::context::AddWidgetMenuContext;
use crate::message::Message;

use super::super::super::components::{menu_item, section_label};
use iced::Theme;
use iced::widget::{Column, rule};

pub(in crate::add_widget_menu::body) fn add_tool_section(
    menu: Column<'static, Message>,
    context: &AddWidgetMenuContext,
    theme: &Theme,
) -> Column<'static, Message> {
    menu.push(rule::horizontal(1))
        .push(section_label("Tools", theme))
        .push(menu_item(
            "Order Book",
            "Pane",
            Some(Message::AddOrderBookPane),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Live Watchlist",
            "Pane",
            Some(Message::AddLiveWatchlistPane),
            context.can_add_pane,
            theme,
        ))
}
