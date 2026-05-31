use super::{BottomTabConfig, PaneKindConfig};
use serde::Deserialize;
use serde::de::IgnoredAny;

#[derive(Deserialize)]
enum KnownPaneKindConfig {
    AccountSummary,
    Chart { chart_id: u64 },
    OrderBook { id: u64 },
    Watchlist,
    LiveWatchlist { id: u64 },
    PositioningInfo { id: u64 },

    Portfolio,
    Income,
    BottomTabs { active_tab: BottomTabConfig },
    OrderEntry,
    AdvancedOrders,
    SpaghettiChart { spaghetti_id: u64 },
    Settings,
    Calendar,
    Liquidations,
    LiquidationsDistribution,
    TrackedTrades,
    TelegramFeed,
    Outcomes,
    HypeEtfs,
    HypeUnstakingQueue,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PaneKindConfigWire {
    Known(KnownPaneKindConfig),
    Unknown(IgnoredAny),
}

impl From<KnownPaneKindConfig> for PaneKindConfig {
    fn from(kind: KnownPaneKindConfig) -> Self {
        match kind {
            KnownPaneKindConfig::AccountSummary => Self::AccountSummary,
            KnownPaneKindConfig::Chart { chart_id } => Self::Chart { chart_id },
            KnownPaneKindConfig::OrderBook { id } => Self::OrderBook { id },
            KnownPaneKindConfig::Watchlist => Self::Watchlist,
            KnownPaneKindConfig::LiveWatchlist { id } => Self::LiveWatchlist { id },
            KnownPaneKindConfig::PositioningInfo { id } => Self::PositioningInfo { id },
            KnownPaneKindConfig::Portfolio => Self::Portfolio,
            KnownPaneKindConfig::Income => Self::Income,
            KnownPaneKindConfig::BottomTabs { active_tab } => Self::BottomTabs { active_tab },
            KnownPaneKindConfig::OrderEntry => Self::OrderEntry,
            KnownPaneKindConfig::AdvancedOrders => Self::AdvancedOrders,
            KnownPaneKindConfig::SpaghettiChart { spaghetti_id } => {
                Self::SpaghettiChart { spaghetti_id }
            }
            KnownPaneKindConfig::Settings => Self::Settings,
            KnownPaneKindConfig::Calendar => Self::Calendar,
            KnownPaneKindConfig::Liquidations => Self::Liquidations,
            KnownPaneKindConfig::LiquidationsDistribution => Self::LiquidationsDistribution,
            KnownPaneKindConfig::TrackedTrades => Self::TrackedTrades,
            KnownPaneKindConfig::TelegramFeed => Self::TelegramFeed,
            KnownPaneKindConfig::Outcomes => Self::Outcomes,
            KnownPaneKindConfig::HypeEtfs => Self::HypeEtfs,
            KnownPaneKindConfig::HypeUnstakingQueue => Self::HypeUnstakingQueue,
        }
    }
}

impl<'de> Deserialize<'de> for PaneKindConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match PaneKindConfigWire::deserialize(deserializer)? {
            PaneKindConfigWire::Known(kind) => kind.into(),
            PaneKindConfigWire::Unknown(_unknown) => Self::Unsupported,
        })
    }
}
