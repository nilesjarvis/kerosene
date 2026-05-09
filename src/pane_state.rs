use crate::account_state::BottomTab;
use crate::chart_state::ChartId;
use crate::market_state::{LiveWatchlistId, OrderBookId};
use crate::spaghetti_state::SpaghettiChartId;

#[derive(Debug, Clone)]
pub(crate) enum PaneKind {
    AccountSummary,
    Chart(ChartId),
    OrderBook(OrderBookId),
    Watchlist,
    LiveWatchlist(LiveWatchlistId),

    Portfolio,
    Income,
    BottomTabs { active_tab: BottomTab },
    OrderEntry,
    SpaghettiChart(SpaghettiChartId),
    Settings,
    Calendar,
    Liquidations,
    TrackedTrades,
    Outcomes,
}
