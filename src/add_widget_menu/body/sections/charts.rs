use super::super::context::AddWidgetMenuContext;
use crate::message::Message;
use crate::pane_management::AddWidgetKind;

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
            Some(Message::BeginWidgetPlacement(
                AddWidgetKind::CandlestickChart,
            )),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Comparison Chart",
            "Pane",
            Some(Message::BeginWidgetPlacement(
                AddWidgetKind::ComparisonChart,
            )),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Pair Ratio",
            "Pane",
            Some(Message::BeginWidgetPlacement(AddWidgetKind::PairRatioChart)),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Session Data",
            "Pane",
            Some(Message::BeginWidgetPlacement(AddWidgetKind::SessionData)),
            context.can_add_pane,
            theme,
        ))
}
