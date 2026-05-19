use crate::account_state::BottomTab;
use crate::chart_state::ChartId;
use crate::market_state::{LiveWatchlistId, OrderBookId};
use crate::positioning_state::PositioningInfoId;
use crate::spaghetti_state::SpaghettiChartId;

pub(crate) const DEFAULT_PANE_BORDER_THICKNESS: f32 = 4.0;
pub(crate) const DEFAULT_PANE_CORNER_RADIUS: f32 = 6.0;

#[derive(Debug, Clone)]
pub(crate) enum PaneKind {
    Chart(ChartId),
    OrderBook(OrderBookId),
    Watchlist,
    LiveWatchlist(LiveWatchlistId),
    PositioningInfo(PositioningInfoId),

    Portfolio,
    Income,
    BottomTabs { active_tab: BottomTab },
    OrderEntry,
    AdvancedOrders,
    SpaghettiChart(SpaghettiChartId),
    Settings,
    Calendar,
    Liquidations,
    TrackedTrades,
    Outcomes,
    HypeEtfs,
}

impl PaneKind {
    pub(crate) fn can_be_closed(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panes_are_closeable() {
        assert!(PaneKind::Chart(0).can_be_closed());
        assert!(PaneKind::OrderEntry.can_be_closed());
    }
}
