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
        PaneKind::Chart(id) => PaneKindConfig::Chart { chart_id: *id },
        PaneKind::OrderBook(id) => PaneKindConfig::OrderBook { id: *id },
        PaneKind::LiveWatchlist(id) => PaneKindConfig::LiveWatchlist { id: *id },
        PaneKind::PositioningInfo(id) => PaneKindConfig::PositioningInfo { id: *id },

        PaneKind::Watchlist => PaneKindConfig::Watchlist,
        PaneKind::Portfolio => PaneKindConfig::Portfolio,
        PaneKind::Income => PaneKindConfig::Income,
        PaneKind::BottomTabs { active_tab } => PaneKindConfig::BottomTabs {
            active_tab: bottom_tab_to_config(*active_tab),
        },
        PaneKind::OrderEntry => PaneKindConfig::OrderEntry,
        PaneKind::AdvancedOrders => PaneKindConfig::AdvancedOrders,
        PaneKind::Settings => PaneKindConfig::Settings,
        PaneKind::SpaghettiChart(id) => PaneKindConfig::SpaghettiChart { spaghetti_id: *id },
        PaneKind::Calendar => PaneKindConfig::Calendar,
        PaneKind::Liquidations => PaneKindConfig::Liquidations,
        PaneKind::LiquidationsDistribution => PaneKindConfig::LiquidationsDistribution,
        PaneKind::TrackedTrades => PaneKindConfig::TrackedTrades,
        PaneKind::Outcomes => PaneKindConfig::Outcomes,
        PaneKind::HypeEtfs => PaneKindConfig::HypeEtfs,
        PaneKind::HypeUnstakingQueue => PaneKindConfig::HypeUnstakingQueue,
    }
}

pub(super) fn pane_kind_from_config(kind: &PaneKindConfig) -> Option<PaneKind> {
    match kind {
        PaneKindConfig::AccountSummary => None,
        PaneKindConfig::Chart { chart_id } => Some(PaneKind::Chart(*chart_id)),
        PaneKindConfig::OrderBook { id } => Some(PaneKind::OrderBook(*id)),
        PaneKindConfig::LiveWatchlist { id } => Some(PaneKind::LiveWatchlist(*id)),
        PaneKindConfig::PositioningInfo { id } => Some(PaneKind::PositioningInfo(*id)),

        PaneKindConfig::Watchlist => Some(PaneKind::Watchlist),
        PaneKindConfig::Portfolio => Some(PaneKind::Portfolio),
        PaneKindConfig::Income => Some(PaneKind::Income),
        PaneKindConfig::Settings => Some(PaneKind::Settings),
        PaneKindConfig::BottomTabs { active_tab } => Some(PaneKind::BottomTabs {
            active_tab: bottom_tab_from_config(*active_tab),
        }),
        PaneKindConfig::OrderEntry => Some(PaneKind::OrderEntry),
        PaneKindConfig::AdvancedOrders => Some(PaneKind::AdvancedOrders),
        PaneKindConfig::SpaghettiChart { spaghetti_id } => {
            Some(PaneKind::SpaghettiChart(*spaghetti_id))
        }
        PaneKindConfig::Calendar => Some(PaneKind::Calendar),
        PaneKindConfig::Liquidations => Some(PaneKind::Liquidations),
        PaneKindConfig::LiquidationsDistribution => Some(PaneKind::LiquidationsDistribution),
        PaneKindConfig::TrackedTrades => Some(PaneKind::TrackedTrades),
        PaneKindConfig::Outcomes => Some(PaneKind::Outcomes),
        PaneKindConfig::HypeEtfs => Some(PaneKind::HypeEtfs),
        PaneKindConfig::HypeUnstakingQueue => Some(PaneKind::HypeUnstakingQueue),
        PaneKindConfig::Unsupported => None,
    }
}
