mod components;
mod controls;
mod groups;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, column, container, responsive, scrollable, text};
use iced::{Element, Fill};
use std::borrow::Cow;

// ---------------------------------------------------------------------------
// Outcome Market Views
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_outcomes(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_outcomes_sized(size.width)).into()
    }

    fn view_outcomes_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let grouped = self.grouped_outcome_markets();
        let searching = !self.outcome_search_query.trim().is_empty();

        let status: Option<Cow<'_, str>> = if self.symbols_loading {
            Some("Loading outcome metadata from Hyperliquid outcomeMeta".into())
        } else if grouped.is_empty() && searching {
            Some(
                format!(
                    "No outcome contracts match \"{}\"",
                    self.outcome_search_query.trim()
                )
                .into(),
            )
        } else if grouped.is_empty() && self.outcome_venue_filter.is_some() {
            Some("No outcome contracts available for this venue".into())
        } else if grouped.is_empty() {
            Some("No outcome contracts returned by Hyperliquid outcomeMeta".into())
        } else if self.outcome_volumes_loading {
            Some("Loading 24h volume".into())
        } else if self.outcome_volumes_error.is_some() {
            Some("24h volume unavailable".into())
        } else {
            None
        };

        let mut market_groups = Column::new().spacing(8);
        let mut is_first_group = true;
        for group in grouped {
            if !is_first_group {
                market_groups = market_groups.push(iced::widget::Space::new().height(2.0));
            }
            is_first_group = false;

            market_groups =
                market_groups.push(self.view_outcome_market_set(&theme, group, available_width));
        }

        let mut content = column![self.view_outcome_controls()].spacing(8);
        if let Some(status) = status {
            content = content.push(
                text(status)
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .width(Fill),
            );
        }
        content = content.push(scrollable(market_groups));

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }
}
