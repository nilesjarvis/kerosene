use super::{BottomTabConfig, PaneKindConfig};
use serde::ser::SerializeStructVariant;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Deserialize)]
enum KnownPaneKindConfig {
    AccountSummary,
    Chart { chart_id: u64 },
    OrderBook { id: u64 },
    Watchlist,
    LiveWatchlist { id: u64 },
    PositioningInfo { id: u64 },
    SessionData { id: u64 },

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
    XFeed,
    Outcomes,
    HypeEtfs,
    HypeUnstakingQueue,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PaneKindConfigWire {
    Known(KnownPaneKindConfig),
    Unknown(serde_json::Value),
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
            KnownPaneKindConfig::SessionData { id } => Self::SessionData { id },
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
            KnownPaneKindConfig::XFeed => Self::XFeed,
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
            PaneKindConfigWire::Unknown(raw) if is_legacy_unsupported_pane(&raw) => {
                Self::Unsupported
            }
            PaneKindConfigWire::Unknown(raw) => Self::Unknown(raw),
        })
    }
}

impl Serialize for PaneKindConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            PaneKindConfig::AccountSummary => {
                serializer.serialize_unit_variant("PaneKindConfig", 0, "AccountSummary")
            }
            PaneKindConfig::Chart { chart_id } => {
                let mut variant =
                    serializer.serialize_struct_variant("PaneKindConfig", 1, "Chart", 1)?;
                variant.serialize_field("chart_id", chart_id)?;
                variant.end()
            }
            PaneKindConfig::OrderBook { id } => {
                let mut variant =
                    serializer.serialize_struct_variant("PaneKindConfig", 2, "OrderBook", 1)?;
                variant.serialize_field("id", id)?;
                variant.end()
            }
            PaneKindConfig::Watchlist => {
                serializer.serialize_unit_variant("PaneKindConfig", 3, "Watchlist")
            }
            PaneKindConfig::LiveWatchlist { id } => {
                let mut variant =
                    serializer.serialize_struct_variant("PaneKindConfig", 4, "LiveWatchlist", 1)?;
                variant.serialize_field("id", id)?;
                variant.end()
            }
            PaneKindConfig::PositioningInfo { id } => {
                let mut variant = serializer.serialize_struct_variant(
                    "PaneKindConfig",
                    5,
                    "PositioningInfo",
                    1,
                )?;
                variant.serialize_field("id", id)?;
                variant.end()
            }
            PaneKindConfig::SessionData { id } => {
                let mut variant =
                    serializer.serialize_struct_variant("PaneKindConfig", 6, "SessionData", 1)?;
                variant.serialize_field("id", id)?;
                variant.end()
            }
            PaneKindConfig::Portfolio => {
                serializer.serialize_unit_variant("PaneKindConfig", 7, "Portfolio")
            }
            PaneKindConfig::Income => {
                serializer.serialize_unit_variant("PaneKindConfig", 8, "Income")
            }
            PaneKindConfig::BottomTabs { active_tab } => {
                let mut variant =
                    serializer.serialize_struct_variant("PaneKindConfig", 9, "BottomTabs", 1)?;
                variant.serialize_field("active_tab", active_tab)?;
                variant.end()
            }
            PaneKindConfig::OrderEntry => {
                serializer.serialize_unit_variant("PaneKindConfig", 10, "OrderEntry")
            }
            PaneKindConfig::AdvancedOrders => {
                serializer.serialize_unit_variant("PaneKindConfig", 11, "AdvancedOrders")
            }
            PaneKindConfig::SpaghettiChart { spaghetti_id } => {
                let mut variant = serializer.serialize_struct_variant(
                    "PaneKindConfig",
                    12,
                    "SpaghettiChart",
                    1,
                )?;
                variant.serialize_field("spaghetti_id", spaghetti_id)?;
                variant.end()
            }
            PaneKindConfig::Settings => {
                serializer.serialize_unit_variant("PaneKindConfig", 13, "Settings")
            }
            PaneKindConfig::Calendar => {
                serializer.serialize_unit_variant("PaneKindConfig", 14, "Calendar")
            }
            PaneKindConfig::Liquidations => {
                serializer.serialize_unit_variant("PaneKindConfig", 15, "Liquidations")
            }
            PaneKindConfig::LiquidationsDistribution => {
                serializer.serialize_unit_variant("PaneKindConfig", 16, "LiquidationsDistribution")
            }
            PaneKindConfig::TrackedTrades => {
                serializer.serialize_unit_variant("PaneKindConfig", 17, "TrackedTrades")
            }
            PaneKindConfig::TelegramFeed => {
                serializer.serialize_unit_variant("PaneKindConfig", 18, "TelegramFeed")
            }
            PaneKindConfig::XFeed => {
                serializer.serialize_unit_variant("PaneKindConfig", 19, "XFeed")
            }
            PaneKindConfig::Outcomes => {
                serializer.serialize_unit_variant("PaneKindConfig", 20, "Outcomes")
            }
            PaneKindConfig::HypeEtfs => {
                serializer.serialize_unit_variant("PaneKindConfig", 21, "HypeEtfs")
            }
            PaneKindConfig::HypeUnstakingQueue => {
                serializer.serialize_unit_variant("PaneKindConfig", 22, "HypeUnstakingQueue")
            }
            PaneKindConfig::Unsupported => {
                serializer.serialize_unit_variant("PaneKindConfig", 23, "Unsupported")
            }
            PaneKindConfig::Unknown(raw) => raw.serialize(serializer),
        }
    }
}

fn is_legacy_unsupported_pane(raw: &serde_json::Value) -> bool {
    matches!(raw.as_str(), Some("Assistant" | "Unsupported"))
}
