mod components;
mod groups;

use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{Column, column, container, responsive, scrollable, text, text_input};
use iced::{Element, Fill};

// ---------------------------------------------------------------------------
// Outcome Market Views
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_outcomes(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_outcomes_sized(size.width)).into()
    }

    fn view_outcomes_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let grouped = self.grouped_outcome_symbols();
        let searching = !self.outcome_search_query.trim().is_empty();

        let status = if self.symbols_loading {
            Some("Loading outcome metadata from Hyperliquid outcomeMeta".to_string())
        } else if grouped.is_empty() && searching {
            Some(format!(
                "No outcome contracts match \"{}\"",
                self.outcome_search_query.trim()
            ))
        } else if grouped.is_empty() {
            Some("No outcome contracts returned by Hyperliquid outcomeMeta".to_string())
        } else if self.outcome_volumes_loading {
            Some("Loading 24h volume".to_string())
        } else if self.outcome_volumes_error.is_some() {
            Some("24h volume unavailable".to_string())
        } else {
            None
        };

        let mut market_groups = Column::new().spacing(8);
        let mut is_first_group = true;
        for (_outcome_id, sides) in grouped {
            if !is_first_group {
                market_groups = market_groups.push(iced::widget::rule::horizontal(1));
            }
            is_first_group = false;

            if let Some(group) = self.view_outcome_market_group(&theme, sides, available_width) {
                market_groups = market_groups.push(group);
            }
        }

        let mut content = column![].spacing(8);
        content = content.push(
            text_input("Search outcome markets...", &self.outcome_search_query)
                .style(helpers::text_input_style)
                .on_input(Message::OutcomeSearchChanged)
                .size(12)
                .padding([5, 8])
                .width(Fill),
        );
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
