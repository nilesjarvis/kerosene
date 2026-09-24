use crate::api::{MarketType, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::Fill;
use iced::widget::{Column, column, pick_list, text_input};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
struct VenueChoice(Option<String>);

impl fmt::Display for VenueChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            self.0
                .as_deref()
                .map(OutcomeSymbolInfo::venue_display_name)
                .unwrap_or("All venues"),
        )
    }
}

impl TradingTerminal {
    pub(super) fn view_outcome_controls(&self) -> Column<'_, Message> {
        let mut venues: BTreeSet<String> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| {
                symbol.market_type == MarketType::Outcome
                    && symbol.is_user_selectable_market()
                    && !self.exchange_symbol_is_hidden(symbol)
            })
            .filter_map(|symbol| symbol.outcome.as_ref()?.venue.clone())
            .collect();
        // Keep the selection visible across metadata outages and market rolls.
        venues.extend(self.outcome_venue_filter.clone());
        let choices: Vec<_> = std::iter::once(VenueChoice(None))
            .chain(venues.into_iter().map(|venue| VenueChoice(Some(venue))))
            .collect();

        column![
            text_input("Search outcome markets...", &self.outcome_search_query)
                .style(helpers::text_input_style)
                .on_input(Message::OutcomeSearchChanged)
                .size(12)
                .padding([5, 8])
                .width(Fill),
            pick_list(
                choices,
                Some(VenueChoice(self.outcome_venue_filter.clone())),
                |choice| Message::OutcomeVenueFilterChanged(choice.0),
            )
            .text_size(11)
            .padding([3, 8])
            .width(Fill),
        ]
        .spacing(4)
    }
}
