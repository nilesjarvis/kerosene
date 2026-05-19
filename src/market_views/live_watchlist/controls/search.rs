use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::LiveWatchlistId;
use crate::message::Message;
use iced::widget::text_input;
use iced::{Element, Fill};

impl TradingTerminal {
    pub(in crate::market_views::live_watchlist) fn view_live_watchlist_search_bar(
        &self,
        id: LiveWatchlistId,
        search_query: &str,
    ) -> Element<'_, Message> {
        text_input("Add symbol...", search_query)
            .style(helpers::text_input_style)
            .on_input(move |q| Message::LiveWatchlistSearchChanged(id, q))
            .size(12)
            .padding([5, 8])
            .width(Fill)
            .into()
    }
}
