use crate::chart_state::ChartId;
use crate::market_state::{LiveWatchlistId, OrderBookId};
use crate::papertrade::PapertradeMarkets;
use crate::positioning_state::PositioningInfoId;
use crate::session_data_state::SessionDataId;
use crate::spaghetti_state::SpaghettiChartId;
use crate::trade_feed_state::TradeFeedId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum MarketVenue {
    #[default]
    Hyperliquid,
    Papertrade,
}

impl MarketVenue {
    pub(crate) const ALL: [Self; 2] = [Self::Hyperliquid, Self::Papertrade];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Hyperliquid => "Hyperliquid",
            Self::Papertrade => "Papertrade",
        }
    }
}

/// A browsing preference belongs to a picker, never to the active trading account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MarketPicker {
    Symbols,
    Chart(ChartId),
    ChartSecondary(ChartId),
    Spaghetti(SpaghettiChartId),
    OrderBook(OrderBookId),
    LiveWatchlist(LiveWatchlistId),
    Positioning(PositioningInfoId),
    SessionData(SessionDataId),
    TradeFeed(TradeFeedId),
    LiquidationDistribution,
}

impl MarketPicker {
    pub(crate) fn chart(id: ChartId, secondary: bool) -> Self {
        if secondary {
            Self::ChartSecondary(id)
        } else {
            Self::Chart(id)
        }
    }
}

#[derive(Default)]
pub(crate) struct MarketBrowserState {
    pub(crate) venues: HashMap<MarketPicker, MarketVenue>,
    pub(crate) papertrade: PapertradeCatalog,
}

impl MarketBrowserState {
    pub(crate) fn venue(&self, picker: MarketPicker) -> MarketVenue {
        self.venues.get(&picker).copied().unwrap_or_default()
    }

    pub(crate) fn clear(&mut self) {
        self.venues.clear();
        // Preserve the generation across config resets so old completions cannot
        // satisfy a new request with the same ID.
        self.papertrade = PapertradeCatalog {
            request_id: self.papertrade.request_id.wrapping_add(1),
            ..Default::default()
        };
    }
}

#[derive(Default)]
pub(crate) struct PapertradeCatalog {
    pub(crate) data: Option<PapertradeMarkets>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    request_id: u64,
    last_attempt: Option<Instant>,
    last_success: Option<Instant>,
}

impl PapertradeCatalog {
    pub(crate) fn begin_request(&mut self, now: Instant, force: bool) -> Option<u64> {
        if self.loading
            || (!force
                && self
                    .last_attempt
                    .is_some_and(|at| now.saturating_duration_since(at) < Duration::from_secs(5)))
            || (!force
                && self.error.is_none()
                && self
                    .last_success
                    .is_some_and(|at| now.saturating_duration_since(at) < Duration::from_secs(60)))
        {
            return None;
        }
        self.request_id = self.request_id.wrapping_add(1);
        self.loading = true;
        self.error = None;
        self.last_attempt = Some(now);
        Some(self.request_id)
    }

    pub(crate) fn finish_request(
        &mut self,
        id: u64,
        result: Result<PapertradeMarkets, String>,
        now: Instant,
    ) {
        if !self.loading || self.request_id != id {
            return;
        }
        self.loading = false;
        match result {
            Ok(data) => {
                self.data = Some(data);
                self.last_success = Some(now);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
}

#[cfg(test)]
mod tests;
