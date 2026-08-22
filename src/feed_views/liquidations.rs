mod controls;
mod footer;
mod layout;
mod rows;

use crate::app_state::TradingTerminal;
use crate::feed_state::liquidation_feed_scroll_id;
use crate::message::Message;
use iced::widget::{column, container, responsive, scrollable};
use iced::{Element, Fill};

const LIQUIDATIONS_CONTENT_HORIZONTAL_PADDING: f32 = 12.0;

impl TradingTerminal {
    pub(crate) fn view_liquidations(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let now_ms = self.status_bar_now_ms;

        if self.hydromancer_api_key.trim().is_empty() {
            return super::feed_empty_state(
                &theme,
                "Add Hydromancer key in Settings > Integrations",
            );
        }

        container(responsive(move |size| {
            self.view_liquidations_sized(
                now_ms,
                (size.width - 2.0 * LIQUIDATIONS_CONTENT_HORIZONTAL_PADDING).max(0.0),
            )
        }))
        .width(Fill)
        .height(Fill)
        .padding(iced::Padding {
            top: 12.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        })
        .into()
    }

    fn view_liquidations_sized(&self, now_ms: u64, available_width: f32) -> Element<'_, Message> {
        let row_layout = layout::LiquidationFeedRowLayout::from_width(available_width);

        // Sticky header: only the rows scroll, header stays visible
        let scroll_content = column![
            iced::widget::rule::horizontal(1),
            self.view_liquidation_feed_rows(now_ms, row_layout),
        ]
        .spacing(0);

        let feed = column![
            self.view_liquidations_top_bar(now_ms),
            self.view_liquidations_header(row_layout),
            scrollable(scroll_content)
                .id(liquidation_feed_scroll_id())
                .on_scroll(Message::LiquidationFeedScrolled)
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .margin(0)
                        .scroller_width(4)
                ))
                .height(Fill),
        ]
        .spacing(8);

        let content = column![
            container(feed)
                .padding([0.0, LIQUIDATIONS_CONTENT_HORIZONTAL_PADDING])
                .width(Fill)
                .height(Fill),
            self.view_liquidations_bottom_content(now_ms),
        ]
        .spacing(0);

        container(content).width(Fill).height(Fill).into()
    }
}
