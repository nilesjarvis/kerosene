use super::{WidgetPaddingOverrideConfig, WidgetPaddingTargetConfig};
use serde::de::{self, IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

#[derive(Deserialize)]
struct WidgetPaddingOverrideWire {
    target: WidgetPaddingTargetConfigWire,
    padding_px: f32,
}

enum WidgetPaddingTargetConfigWire {
    Known(WidgetPaddingTargetConfig),
    Unknown,
}

impl<'de> Deserialize<'de> for WidgetPaddingTargetConfigWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(WidgetPaddingTargetVisitor)
    }
}

struct WidgetPaddingTargetVisitor;

impl<'de> Visitor<'de> for WidgetPaddingTargetVisitor {
    type Value = WidgetPaddingTargetConfigWire;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a widget padding target")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "Watchlist" => Ok(known(WidgetPaddingTargetConfig::Watchlist)),
            "Portfolio" => Ok(known(WidgetPaddingTargetConfig::Portfolio)),
            "Income" => Ok(known(WidgetPaddingTargetConfig::Income)),
            "BottomTabs" => Ok(known(WidgetPaddingTargetConfig::BottomTabs)),
            "OrderEntry" => Ok(known(WidgetPaddingTargetConfig::OrderEntry)),
            "AdvancedOrders" => Ok(known(WidgetPaddingTargetConfig::AdvancedOrders)),
            "Settings" => Ok(known(WidgetPaddingTargetConfig::Settings)),
            "Calendar" => Ok(known(WidgetPaddingTargetConfig::Calendar)),
            "Liquidations" => Ok(known(WidgetPaddingTargetConfig::Liquidations)),
            "LiquidationsDistribution" => {
                Ok(known(WidgetPaddingTargetConfig::LiquidationsDistribution))
            }
            "TrackedTrades" => Ok(known(WidgetPaddingTargetConfig::TrackedTrades)),
            "TelegramFeed" => Ok(known(WidgetPaddingTargetConfig::TelegramFeed)),
            "XFeed" => Ok(known(WidgetPaddingTargetConfig::XFeed)),
            "Outcomes" => Ok(known(WidgetPaddingTargetConfig::Outcomes)),
            "HypeEtfs" => Ok(known(WidgetPaddingTargetConfig::HypeEtfs)),
            "HypeUnstakingQueue" => Ok(known(WidgetPaddingTargetConfig::HypeUnstakingQueue)),
            "Chart" | "OrderBook" | "LiveWatchlist" | "PositioningInfo" | "SessionData"
            | "SpaghettiChart" => Err(E::custom(format!(
                "widget padding target '{value}' requires a payload"
            ))),
            _ => Ok(WidgetPaddingTargetConfigWire::Unknown),
        }
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let Some(target_name) = map.next_key::<String>()? else {
            return Err(de::Error::invalid_length(0, &self));
        };

        let target = match target_name.as_str() {
            "Chart" => {
                #[derive(Deserialize)]
                struct Payload {
                    chart_id: u64,
                }
                let payload = map.next_value::<Payload>()?;
                known(WidgetPaddingTargetConfig::Chart {
                    chart_id: payload.chart_id,
                })
            }
            "OrderBook" => {
                #[derive(Deserialize)]
                struct Payload {
                    id: u64,
                }
                let payload = map.next_value::<Payload>()?;
                known(WidgetPaddingTargetConfig::OrderBook { id: payload.id })
            }
            "LiveWatchlist" => {
                #[derive(Deserialize)]
                struct Payload {
                    id: u64,
                }
                let payload = map.next_value::<Payload>()?;
                known(WidgetPaddingTargetConfig::LiveWatchlist { id: payload.id })
            }
            "PositioningInfo" => {
                #[derive(Deserialize)]
                struct Payload {
                    id: u64,
                }
                let payload = map.next_value::<Payload>()?;
                known(WidgetPaddingTargetConfig::PositioningInfo { id: payload.id })
            }
            "SessionData" => {
                #[derive(Deserialize)]
                struct Payload {
                    id: u64,
                }
                let payload = map.next_value::<Payload>()?;
                known(WidgetPaddingTargetConfig::SessionData { id: payload.id })
            }
            "SpaghettiChart" => {
                #[derive(Deserialize)]
                struct Payload {
                    spaghetti_id: u64,
                }
                let payload = map.next_value::<Payload>()?;
                known(WidgetPaddingTargetConfig::SpaghettiChart {
                    spaghetti_id: payload.spaghetti_id,
                })
            }
            "Watchlist"
            | "Portfolio"
            | "Income"
            | "BottomTabs"
            | "OrderEntry"
            | "AdvancedOrders"
            | "Settings"
            | "Calendar"
            | "Liquidations"
            | "LiquidationsDistribution"
            | "TrackedTrades"
            | "TelegramFeed"
            | "XFeed"
            | "Outcomes"
            | "HypeEtfs"
            | "HypeUnstakingQueue" => {
                return Err(de::Error::custom(format!(
                    "widget padding target '{target_name}' does not take a payload"
                )));
            }
            _ => {
                let _ = map.next_value::<IgnoredAny>()?;
                if map.next_key::<IgnoredAny>()?.is_some() {
                    return Err(de::Error::custom(
                        "widget padding target must contain exactly one variant",
                    ));
                }
                return Ok(WidgetPaddingTargetConfigWire::Unknown);
            }
        };

        if map.next_key::<IgnoredAny>()?.is_some() {
            return Err(de::Error::custom(
                "widget padding target must contain exactly one variant",
            ));
        }

        Ok(target)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Err(E::custom("widget padding target cannot be null"))
    }
}

pub(super) fn deserialize_overrides<'de, D>(
    deserializer: D,
) -> Result<Vec<WidgetPaddingOverrideConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    let overrides = Vec::<WidgetPaddingOverrideWire>::deserialize(deserializer)?;
    Ok(overrides
        .into_iter()
        .filter_map(|item| {
            let WidgetPaddingTargetConfigWire::Known(target) = item.target else {
                return None;
            };

            Some(WidgetPaddingOverrideConfig {
                target,
                padding_px: item.padding_px,
            })
        })
        .collect())
}

fn known(target: WidgetPaddingTargetConfig) -> WidgetPaddingTargetConfigWire {
    WidgetPaddingTargetConfigWire::Known(target)
}
