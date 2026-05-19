mod components;
mod groups;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, column, container, responsive, row, scrollable, text};
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

        let mut status = if self.symbols_loading {
            "Loading outcome metadata from Hyperliquid outcomeMeta"
        } else if grouped.is_empty() {
            "No outcome contracts returned by Hyperliquid outcomeMeta"
        } else {
            "Read-only USDH outcomes - 24h volume from candleSnapshot"
        }
        .to_string();
        if self.outcome_volumes_loading {
            status.push_str(" - loading volume");
        } else if self.outcome_volumes_error.is_some() {
            status.push_str(" - volume unavailable");
        }

        let status_row = row![
            text(status)
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(Fill)
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

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

        let content = column![status_row, scrollable(market_groups)].spacing(8);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }
}
