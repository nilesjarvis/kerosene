use super::super::context::AddWidgetMenuContext;
use crate::message::Message;

use super::super::super::components::{menu_item, section_label};
use iced::Theme;
use iced::widget::{Column, rule};

pub(in crate::add_widget_menu::body) fn add_chart_section(
    menu: Column<'static, Message>,
    context: &AddWidgetMenuContext,
    theme: &Theme,
) -> Column<'static, Message> {
    menu.push(rule::horizontal(1))
        .push(section_label("Charts", theme))
        .push(menu_item(
            "Candlestick Chart",
            "Pane",
            context.target.map(Message::AddChart),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Comparison Chart",
            "Pane",
            Some(Message::AddComparisonChart),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Pair Ratio",
            "Pane",
            Some(Message::AddPairRatioChart),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Session Data",
            "Pane",
            Some(Message::AddSessionDataPane),
            context.can_add_pane,
            theme,
        ))
}
