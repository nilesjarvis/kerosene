use crate::app_state::TradingTerminal;
use crate::market_state::market_browser::{MarketPicker, MarketVenue};
use crate::message::Message;
use iced::Task;
use std::time::Instant;

impl TradingTerminal {
    pub(super) fn update_market_browser(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MarketPickerVenueChanged(picker, venue) => {
                if !self.market_picker_exists(picker) {
                    return Task::none();
                }
                let mut venues = std::mem::take(&mut self.market_browser.venues);
                venues.retain(|picker, _| self.market_picker_exists(*picker));
                if venue == MarketVenue::Hyperliquid {
                    venues.remove(&picker);
                } else {
                    venues.insert(picker, venue);
                }
                self.market_browser.venues = venues;

                let mut tasks = Vec::new();
                if let MarketPicker::Chart(id) | MarketPicker::ChartSecondary(id) = picker {
                    let secondary = matches!(picker, MarketPicker::ChartSecondary(_));
                    if let Some(instance) = self.charts.get_mut(&id) {
                        if secondary {
                            instance.secondary_editor_selected_index = None;
                        } else {
                            instance.editor_selected_index = None;
                        }
                    }
                    tasks.push(if secondary {
                        Task::batch([
                            Self::focus_chart_secondary_symbol_search_input(id),
                            Self::scroll_chart_secondary_symbol_search_results_to(id, 0.0),
                        ])
                    } else {
                        Task::batch([
                            Self::focus_chart_symbol_search_input(id),
                            Self::scroll_chart_symbol_search_results_to(id, 0.0),
                        ])
                    });
                }
                if venue == MarketVenue::Papertrade {
                    tasks.push(self.request_papertrade_markets(false));
                }
                Task::batch(tasks)
            }
            Message::RefreshPapertradeMarkets => self.request_papertrade_markets(true),
            Message::PapertradeMarketsLoaded(id, result) => {
                self.market_browser
                    .papertrade
                    .finish_request(id, *result, Instant::now());
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn request_papertrade_markets(&mut self, force: bool) -> Task<Message> {
        let Some(id) = self
            .market_browser
            .papertrade
            .begin_request(Instant::now(), force)
        else {
            return Task::none();
        };
        Task::perform(crate::papertrade::fetch_markets(), move |result| {
            Message::PapertradeMarketsLoaded(id, Box::new(result))
        })
    }

    fn market_picker_exists(&self, picker: MarketPicker) -> bool {
        match picker {
            MarketPicker::Symbols | MarketPicker::LiquidationDistribution => true,
            MarketPicker::Chart(id) | MarketPicker::ChartSecondary(id) => {
                self.charts.contains_key(&id)
            }
            MarketPicker::Spaghetti(id) => self.spaghetti_charts.contains_key(&id),
            MarketPicker::OrderBook(id) => self.order_books.contains_key(&id),
            MarketPicker::LiveWatchlist(id) => self.live_watchlists.contains_key(&id),
            MarketPicker::Positioning(id) => self.positioning_infos.contains_key(&id),
            MarketPicker::SessionData(id) => self.session_data.contains_key(&id),
            MarketPicker::TradeFeed(id) => self.trade_feeds.instances.contains_key(&id),
        }
    }
}

#[cfg(test)]
mod tests;
