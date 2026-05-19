mod hype_etfs;
mod live_watchlist;
mod order_book;
mod positioning_info;
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
            | Message::OutcomeVolumesLoaded(_)
            | Message::SymbolSelected(_)) => {
                return self.update_symbol_search_market(message);
            }
            message @ (Message::RefreshHypeEtfs
            | Message::HypeEtfsRefreshTick
            | Message::HypeEtfsViewChanged(_)
            | Message::HypeEtfsLoaded(_)) => {
                return self.update_hype_etfs_market(message);
            }
            message if is_live_watchlist_market_message(&message) => {
                return self.update_live_watchlist_market(message);
            }
            message if is_order_book_market_message(&message) => {
                return self.update_order_book_market(message);
            }
            message if is_positioning_info_market_message(&message) => {
                return self.update_positioning_info_market(message);
            }
            _ => {}
        }

        Task::none()
    }
}

fn is_live_watchlist_market_message(message: &Message) -> bool {
    matches!(
        message,
        Message::LiveWatchlistSortChanged(_, _)
            | Message::LiveWatchlistColumnToggled(_, _, _)
            | Message::ToggleLiveWatchlistSettings(_)
            | Message::AddLiveWatchlistPane
            | Message::LiveWatchlistSearchChanged(_, _)
            | Message::LiveWatchlistAddSymbol(_, _)
            | Message::LiveWatchlistRemoveSymbol(_, _)
            | Message::LiveWatchlistRefreshTick
            | Message::LiveWatchlistContextsLoaded(_, _)
            | Message::LiveWatchlistHistoryLoaded(_, _, _)
    )
}

fn is_positioning_info_market_message(message: &Message) -> bool {
    matches!(
        message,
        Message::AddPositioningInfoPane
            | Message::PositioningInfoPageChanged(_, _)
            | Message::PositioningInfoSearchChanged(_, _)
            | Message::PositioningInfoSymbolSelected(_, _)
            | Message::PositioningInfoSideChanged(_, _)
            | Message::PositioningInfoSortChanged(_, _)
            | Message::PositioningInfoChangeTimeframeChanged(_, _)
            | Message::PositioningInfoChangeSortChanged(_, _)
            | Message::ClearPositioningInfoFilters(_)
            | Message::RefreshPositioningInfoPane(_)
            | Message::RefreshPositioningInfo
            | Message::PositioningInfoWsAssetCtxUpdate(_, _)
            | Message::PositioningInfoLoaded(_, _)
            | Message::PositioningInfoChangeLoaded(_, _)
    )
}

fn is_order_book_market_message(message: &Message) -> bool {
    matches!(
        message,
        Message::AddOrderBookPane
            | Message::BookLoaded { .. }
            | Message::OrderBookWsAssetCtxUpdate(_, _)
            | Message::WsBookUpdate { .. }
            | Message::SetBookTickSize(_, _)
            | Message::ToggleOrderBookSettings(_)
            | Message::ToggleOrderBookCenterOnMid(_)
            | Message::ToggleOrderBookReverseSide(_)
            | Message::ToggleOrderBookSpreadChart(_)
            | Message::OrderBookSpreadChartResize(_, _)
            | Message::OrderBookSearchChanged(_, _)
            | Message::OrderBookSetMode(_, _)
            | Message::SetOrderBookDisplayMode(_, _)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_state::OrderBookDisplayMode;

    #[test]
    fn order_book_market_dispatch_includes_order_book_controls() {
        assert!(is_order_book_market_message(
            &Message::SetOrderBookDisplayMode(7, OrderBookDisplayMode::DomLadder)
        ));
        assert!(is_order_book_market_message(
            &Message::ToggleOrderBookCenterOnMid(7)
        ));
        assert!(is_order_book_market_message(
            &Message::ToggleOrderBookReverseSide(7)
        ));
    }

    #[test]
    fn live_watchlist_market_dispatch_includes_settings_toggle() {
        assert!(is_live_watchlist_market_message(
            &Message::ToggleLiveWatchlistSettings(7)
        ));
    }
}
