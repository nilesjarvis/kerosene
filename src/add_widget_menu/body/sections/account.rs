use super::super::context::AddWidgetMenuContext;
use crate::message::Message;

use super::super::super::components::{menu_item, section_label};
use iced::Theme;
use iced::widget::{Column, rule};

pub(in crate::add_widget_menu::body) fn add_account_section(
    menu: Column<'static, Message>,
    context: &AddWidgetMenuContext,
    theme: &Theme,
) -> Column<'static, Message> {
    menu.push(rule::horizontal(1))
        .push(section_label("Account Panes", theme))
        .push(menu_item(
            "Portfolio",
            if context.portfolio_open {
                "Open"
            } else {
                "Pane"
            },
            Some(Message::AddPortfolioPane),
            context.can_add_pane,
            theme,
        ))
        .push(menu_item(
            "Income",
            income_tag(context),
            (context.income_open || context.can_add_income).then_some(Message::AddIncomePane),
            context.can_add_pane && (context.income_open || context.can_add_income),
            theme,
        ))
}

fn income_tag(context: &AddWidgetMenuContext) -> &'static str {
    if context.income_open {
        "Open"
    } else if context.can_add_income {
        "Pane"
    } else {
        "Requires PM"
    }
}
