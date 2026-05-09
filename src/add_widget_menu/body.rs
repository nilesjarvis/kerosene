mod context;
mod header;
mod sections;

use self::context::AddWidgetMenuContext;
use self::header::{placement_controls, target_header};
use self::sections::{
    add_account_section, add_chart_section, add_feed_section, add_tool_section, add_window_section,
};

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::Column;
use iced::{Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_add_widget_menu_body(
        &self,
        theme: &Theme,
        can_add_income: bool,
    ) -> Column<'static, Message> {
        let context = AddWidgetMenuContext::new(self, can_add_income);
        let menu = Column::new()
            .spacing(2)
            .width(Fill)
            .push(target_header(&context, theme))
            .push(placement_controls(context.placement));
        let menu = add_chart_section(menu, &context, theme);
        let menu = add_account_section(menu, &context, theme);
        let menu = add_feed_section(menu, &context, theme);
        let menu = add_tool_section(menu, &context, theme);

        add_window_section(menu, &context, theme)
    }
}
