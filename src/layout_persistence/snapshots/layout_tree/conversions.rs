use crate::account_state::BottomTab;
use crate::config::{BottomTabConfig, PaneKindConfig};
use crate::pane_state::PaneKind;

// ---------------------------------------------------------------------------
// Pane Kind Conversion
// ---------------------------------------------------------------------------

fn bottom_tab_to_config(tab: BottomTab) -> BottomTabConfig {
    match tab {
        BottomTab::Positions => BottomTabConfig::Positions,
        BottomTab::OpenOrders => BottomTabConfig::OpenOrders,
        BottomTab::Balances => BottomTabConfig::Balances,
        BottomTab::TradeHistory => BottomTabConfig::TradeHistory,
        BottomTab::FundingHistory => BottomTabConfig::FundingHistory,
    }
}

fn bottom_tab_from_config(tab: BottomTabConfig) -> BottomTab {
    match tab {
        BottomTabConfig::Positions => BottomTab::Positions,
        BottomTabConfig::OpenOrders => BottomTab::OpenOrders,
        BottomTabConfig::Balances => BottomTab::Balances,
        BottomTabConfig::TradeHistory => BottomTab::TradeHistory,
        BottomTabConfig::FundingHistory => BottomTab::FundingHistory,
    }
}

pub(super) fn pane_kind_to_config(kind: &PaneKind) -> PaneKindConfig {
    match kind {
        PaneKind::AccountSummary => PaneKindConfig::AccountSummary,
        PaneKind::Chart(id) => PaneKindConfig::Chart { chart_id: *id },
        PaneKind::OrderBook(id) => PaneKindConfig::OrderBook { id: *id },
        PaneKind::LiveWatchlist(id) => PaneKindConfig::LiveWatchlist { id: *id },

        PaneKind::Watchlist => PaneKindConfig::Watchlist,
        PaneKind::Portfolio => PaneKindConfig::Portfolio,
        PaneKind::Income => PaneKindConfig::Income,
        PaneKind::Assistant => PaneKindConfig::Assistant,
        PaneKind::BottomTabs { active_tab } => PaneKindConfig::BottomTabs {
            active_tab: bottom_tab_to_config(*active_tab),
        },
        PaneKind::OrderEntry => PaneKindConfig::OrderEntry,
        PaneKind::Settings => PaneKindConfig::Settings,
        PaneKind::SpaghettiChart(id) => PaneKindConfig::SpaghettiChart { spaghetti_id: *id },
        PaneKind::Calendar => PaneKindConfig::Calendar,
        PaneKind::Liquidations => PaneKindConfig::Liquidations,
        PaneKind::TrackedTrades => PaneKindConfig::TrackedTrades,
        PaneKind::Outcomes => PaneKindConfig::Outcomes,
    }
}

pub(super) fn pane_kind_from_config(kind: &PaneKindConfig) -> PaneKind {
    match kind {
        PaneKindConfig::AccountSummary => PaneKind::AccountSummary,
        PaneKindConfig::Chart { chart_id } => PaneKind::Chart(*chart_id),
        PaneKindConfig::OrderBook { id } => PaneKind::OrderBook(*id),
        PaneKindConfig::LiveWatchlist { id } => PaneKind::LiveWatchlist(*id),

        PaneKindConfig::Watchlist => PaneKind::Watchlist,
        PaneKindConfig::Portfolio => PaneKind::Portfolio,
        PaneKindConfig::Income => PaneKind::Income,
        PaneKindConfig::Assistant => PaneKind::Assistant,
        PaneKindConfig::Settings => PaneKind::Settings,
        PaneKindConfig::BottomTabs { active_tab } => PaneKind::BottomTabs {
            active_tab: bottom_tab_from_config(*active_tab),
        },
        PaneKindConfig::OrderEntry => PaneKind::OrderEntry,
        PaneKindConfig::SpaghettiChart { spaghetti_id } => PaneKind::SpaghettiChart(*spaghetti_id),
        PaneKindConfig::Calendar => PaneKind::Calendar,
        PaneKindConfig::Liquidations => PaneKind::Liquidations,
        PaneKindConfig::TrackedTrades => PaneKind::TrackedTrades,
        PaneKindConfig::Outcomes => PaneKind::Outcomes,
    }
}
