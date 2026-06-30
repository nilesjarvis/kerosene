use super::super::context::AddWidgetMenuContext;
use crate::message::Message;

use super::super::super::components::{menu_item, section_label};
use iced::Theme;
use iced::widget::{Column, rule};

pub(in crate::add_widget_menu::body) fn add_feed_section(
    menu: Column<'static, Message>,
    context: &AddWidgetMenuContext,
    theme: &Theme,
) -> Column<'static, Message> {
    menu.push(rule::horizontal(1))
        .push(section_label("Feeds", theme))
        .push(menu_item(
            "Outcomes",
            if context.outcomes_open {
                "Open"
            } else {
                "Pane"
            },
            Some(Message::AddOutcomesPane),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "HYPE ETFs",
            if context.hype_etfs_open {
                "Open"
            } else {
                "Pane"
            },
            Some(Message::AddHypeEtfsPane),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "HYPE Unstaking Queue",
            if context.hype_unstaking_queue_open {
                "Open"
            } else {
                "Pane"
            },
            Some(Message::AddHypeUnstakingQueuePane),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Liquidations Feed",
            if context.liquidations_open {
                "Open"
            } else {
                "Pane"
            },
            Some(Message::AddLiquidationsPane),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Liquidations Distribution",
            if context.liquidations_distribution_open {
                "Open"
            } else {
                "Pane"
            },
            Some(Message::AddLiquidationsDistributionPane),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Wallet Tracker",
            if context.tracked_trades_open {
                "Open"
            } else {
                "Pane"
            },
            Some(Message::AddTrackedTradesPane),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Telegram Feed",
            if context.telegram_feed_open {
                "Open"
            } else {
                "Pane"
            },
            Some(Message::AddTelegramFeedPane),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "X Feed",
            "Pane",
            Some(Message::AddXFeedPane),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Calendar",
            if context.calendar_open {
                "Open"
            } else {
                "Pane"
            },
            Some(Message::AddCalendarPane),
            context.can_add_pane,
            theme,
        ))
}
