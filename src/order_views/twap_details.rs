use self::sections::{twap_child_orders, twap_events, twap_header, twap_notes, twap_summary};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{column, container, rule, scrollable, text};
use iced::{Element, Fill, Theme, window};

mod formatting;
mod sections;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// TWAP Details Window
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_twap_details(&self, window_id: window::Id) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(twap) = self
            .twap_orders
            .values()
            .find(|twap| twap.window_id == Some(window_id))
        else {
            return container(text("TWAP not found").size(13))
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .into();
        };

        let content = column![
            twap_header(twap, &theme),
            rule::horizontal(1),
            twap_summary(twap, self.status_bar_now, &theme),
            rule::horizontal(1),
            twap_child_orders(twap, &theme),
            rule::horizontal(1),
            twap_events(twap, &theme),
            rule::horizontal(1),
            twap_notes(&theme),
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
