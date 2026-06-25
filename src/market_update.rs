mod hype_etfs;
mod hype_unstaking_queue;
mod live_watchlist;
mod order_book;
mod positioning_info;
mod session_data;
mod symbols;
mod ticker_tape;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_market(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ (Message::ToggleFavourite(_)
            | Message::SymbolsLoaded(_)
            | Message::ExchangeSymbolsRefreshTick
            | Message::SymbolSearchChanged(_)
            | Message::SymbolSearchSortChanged(_)
            | Message::SymbolSearchMarketFilterChanged(_)
            | Message::SymbolSearchHip3DexFilterChanged(_)
            | Message::SymbolSearchContextsLoaded(_, _, _, _)
            | Message::OutcomeSearchChanged(_)
            | Message::OutcomeMarketGroupToggled(_)
            | Message::OutcomeVolumesLoaded(_, _, _)
            | Message::SymbolSelected(_)) => {
                return self.update_symbol_search_market(message);
            }
            message @ (Message::RefreshHypeEtfs
            | Message::HypeEtfsRefreshTick
            | Message::HypeEtfsViewChanged(_)
            | Message::HypeEtfsLoaded(_, _)) => {
                return self.update_hype_etfs_market(message);
            }
            message if is_hype_unstaking_queue_market_message(&message) => {
                return self.update_hype_unstaking_queue_market(message);
            }
            message @ (Message::TickerTapeRefreshTick
            | Message::TickerTapeContextsLoaded(_, _, _, _)) => {
                return self.update_ticker_tape_market(message);
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
            message if is_session_data_market_message(&message) => {
                return self.update_session_data_market(message);
            }
            _ => {}
        }

        Task::none()
    }
}

fn is_hype_unstaking_queue_market_message(message: &Message) -> bool {
    matches!(
        message,
        Message::RefreshHypeUnstakingQueue
            | Message::HypeUnstakingQueueRefreshTick
            | Message::HypeUnstakingWindowChanged(_)
            | Message::HypeUnstakingAmountFilterChanged(_)
            | Message::HypeUnstakingSortChanged(_)
            | Message::ToggleHypeUnstakingMineOnly
            | Message::ClearHypeUnstakingFilters
            | Message::HypeUnstakingQueueLoaded(_, _)
    )
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
            | Message::LiveWatchlistContextsLoaded(_, _, _, _)
            | Message::LiveWatchlistHistoryLoaded(_, _, _, _)
    )
}

fn is_positioning_info_market_message(message: &Message) -> bool {
    matches!(
        message,
        Message::AddPositioningInfoPane
            | Message::PositioningInfoPageChanged(_, _)
            | Message::PositioningInfoSearchChanged(_, _)
            | Message::TogglePositioningInfoSymbolPicker(_)
            | Message::PositioningInfoSymbolSelected(_, _)
            | Message::PositioningInfoSideChanged(_, _)
            | Message::PositioningInfoSortChanged(_, _)
            | Message::PositioningInfoEntryMinChanged(_, _)
            | Message::PositioningInfoEntryMaxChanged(_, _)
            | Message::ApplyPositioningInfoEntryRange(_)
            | Message::PositioningInfoChangeTimeframeChanged(_, _)
            | Message::ClearPositioningInfoFilters(_)
            | Message::RefreshPositioningInfoPane(_)
            | Message::RefreshPositioningInfo
            | Message::PositioningInfoWsAssetCtxUpdate(_, _, _)
            | Message::PositioningInfoWsAssetCtxLagged(_, _, _)
            | Message::PositioningInfoLoaded(_, _, _)
            | Message::PositioningInfoChangeLoaded(_, _, _)
    )
}

fn is_order_book_market_message(message: &Message) -> bool {
    matches!(
        message,
        Message::AddOrderBookPane
            | Message::BookLoaded { .. }
            | Message::OrderBookWsAssetCtxUpdate { .. }
            | Message::OrderBookWsAssetCtxLagged { .. }
            | Message::WsBookUpdate { .. }
            | Message::OrderBookWsBookLagged { .. }
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

fn is_session_data_market_message(message: &Message) -> bool {
    matches!(
        message,
        Message::AddSessionDataPane
            | Message::SessionDataSearchChanged(_, _)
            | Message::ToggleSessionDataSymbolPicker(_)
            | Message::SessionDataSymbolSelected(_, _)
            | Message::SessionDataLookbackChanged(_, _)
            | Message::RefreshSessionData(_)
            | Message::SessionDataCandlesLoaded(_, _)
    )
}

#[cfg(test)]
mod tests;
