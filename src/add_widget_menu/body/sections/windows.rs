use super::super::context::AddWidgetMenuContext;
use crate::message::Message;

use super::super::super::components::{menu_item, section_label};
use iced::Theme;
use iced::widget::{Column, rule};

pub(in crate::add_widget_menu::body) fn add_window_section(
    menu: Column<'static, Message>,
    context: &AddWidgetMenuContext,
    theme: &Theme,
) -> Column<'static, Message> {
    menu.push(rule::horizontal(1))
        .push(section_label("Windows", theme))
        .push(menu_item(
            "Trading Journal",
            if context.journal_open {
                "Open"
            } else {
                "Window"
            },
            Some(Message::AddTradingJournal),
            true,
            theme,
        ))
        .push(menu_item(
            "Wallet Tracker",
            if context.wallet_tracker_open {
                "Open"
            } else {
                "Window"
            },
            Some(Message::OpenWalletTrackerWindow),
            true,
            theme,
        ))
        .push(menu_item(
            "Screener",
            if context.screener_open {
                "Open"
            } else {
                "Window"
            },
            Some(Message::OpenScreenerWindow),
            true,
            theme,
        ))
        .push(menu_item(
            "Settings",
            if context.settings_open {
                "Open"
            } else {
                "Window"
            },
            Some(Message::OpenSettingsWindow),
            true,
            theme,
        ))
}
