mod live_watchlist;
mod order_book;
mod symbols;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_market(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ (Message::ToggleFavourite(_)
            | Message::SymbolsLoaded(_)
            | Message::SymbolSearchChanged(_)
            | Message::SymbolSearchSortChanged(_)
            | Message::SymbolSearchMarketFilterChanged(_)
            | Message::SymbolSearchHip3DexFilterChanged(_)
            | Message::SymbolSearchContextsLoaded(_, _)
            | Message::SymbolSelected(_)) => {
                return self.update_symbol_search_market(message);
            }
            message @ (Message::LiveWatchlistSortChanged(_, _)
            | Message::LiveWatchlistColumnToggled(_, _, _)
            | Message::AddLiveWatchlistPane
            | Message::LiveWatchlistSearchChanged(_, _)
            | Message::LiveWatchlistAddSymbol(_, _)
            | Message::LiveWatchlistRemoveSymbol(_, _)
            | Message::LiveWatchlistRefreshTick
            | Message::LiveWatchlistContextsLoaded(_, _)
            | Message::LiveWatchlistHistoryLoaded(_, _, _)) => {
                return self.update_live_watchlist_market(message);
            }
            message if is_order_book_market_message(&message) => {
                return self.update_order_book_market(message);
            }
            _ => {}
        }

        Task::none()
    }
}

fn is_order_book_market_message(message: &Message) -> bool {
    matches!(
        message,
        Message::AddOrderBookPane
            | Message::BookLoaded(_, _)
            | Message::OrderBookWsAssetCtxUpdate(_, _)
            | Message::WsBookUpdate(_, _, _)
            | Message::SetBookTickSize(_, _)
            | Message::ToggleOrderBookSettings(_)
            | Message::ToggleOrderBookSpreadChart(_)
            | Message::OrderBookSpreadChartResize(_, _)
            | Message::OrderBookSearchChanged(_, _)
            | Message::OrderBookSetMode(_, _)
            | Message::SetOrderBookDisplayMode(_, _)
            | Message::CenterOrderBook(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_state::OrderBookDisplayMode;

    #[test]
    fn order_book_market_dispatch_includes_display_mode_switches() {
        assert!(is_order_book_market_message(
            &Message::SetOrderBookDisplayMode(7, OrderBookDisplayMode::DomLadder)
        ));
    }
}
