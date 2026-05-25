use self::sections::{history_children, history_header, history_logs, history_summary};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{column, container, rule, scrollable, text};
use iced::{Element, Fill, Theme, window};

mod formatting;
mod sections;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Advanced Order History Details
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_advanced_order_history_details(
        &self,
        window_id: window::Id,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(entry_id) = self.advanced_order_history_windows.get(&window_id) else {
            return missing_history_view();
        };
        let Some(entry) = self
            .advanced_order_history
            .iter()
            .find(|entry| entry.id == *entry_id)
        else {
            return missing_history_view();
        };

        let content = column![
            history_header(entry, &theme),
            rule::horizontal(1),
            history_summary(entry, &theme),
            rule::horizontal(1),
            history_children(entry, &theme),
            rule::horizontal(1),
            history_logs(entry, &theme),
        ]
        .spacing(10)
        .padding(12);

        container(
            scrollable(content).direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .margin(0)
                    .scroller_width(4),
            )),
        )
        .width(Fill)
        .height(Fill)
        .style(|theme: &Theme| iced::widget::container::Style {
            background: Some(theme.extended_palette().background.base.color.into()),
            text_color: Some(theme.palette().text),
            ..Default::default()
        })
        .into()
    }
}

fn missing_history_view<'a>() -> Element<'a, Message> {
    container(text("Advanced order history not found").size(13))
        .width(Fill)
        .height(Fill)
        .center(Fill)
        .into()
}
