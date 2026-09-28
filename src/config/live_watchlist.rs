use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
pub enum LiveWatchlistSortColumn {
    #[default]
    Symbol,
    Price,
    Change5m,
    Change30m,
    Change1h,
    Change24h,
    Funding,
    EmaDistance,
}

impl LiveWatchlistSortColumn {
    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "Symbol" => Some(Self::Symbol),
            "Price" => Some(Self::Price),
            "Change5m" => Some(Self::Change5m),
            "Change30m" => Some(Self::Change30m),
            "Change1h" => Some(Self::Change1h),
            "Change24h" => Some(Self::Change24h),
            "Funding" => Some(Self::Funding),
            "EmaDistance" => Some(Self::EmaDistance),
            _ => None,
        }
    }

    fn config_value(self) -> &'static str {
        match self {
            Self::Symbol => "Symbol",
            Self::Price => "Price",
            Self::Change5m => "Change5m",
            Self::Change30m => "Change30m",
            Self::Change1h => "Change1h",
            Self::Change24h => "Change24h",
            Self::Funding => "Funding",
            Self::EmaDistance => "EmaDistance",
        }
    }
}

impl<'de> Deserialize<'de> for LiveWatchlistSortColumn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_config_value(&value).unwrap_or_else(|| {
            let default = Self::default();
            push_unknown_live_watchlist_sort_value_warning(
                "sort column",
                &value,
                default.config_value(),
            );
            default
        }))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LiveWatchlistColumn {
    Price,
    Change5m,
    Change30m,
    Change1h,
    Change24h,
    Funding,
    EmaDistance,
}

impl LiveWatchlistColumn {
    pub const ALL: [Self; 7] = [
        Self::Price,
        Self::EmaDistance,
        Self::Change5m,
        Self::Change30m,
        Self::Change1h,
        Self::Change24h,
        Self::Funding,
    ];

    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "Price" => Some(Self::Price),
            "Change5m" => Some(Self::Change5m),
            "Change30m" => Some(Self::Change30m),
            "Change1h" => Some(Self::Change1h),
            "Change24h" => Some(Self::Change24h),
            "Funding" => Some(Self::Funding),
            "EmaDistance" => Some(Self::EmaDistance),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Price => "Price",
            Self::Change5m => "5m",
            Self::Change30m => "30m",
            Self::Change1h => "1h",
            Self::Change24h => "24h",
            Self::Funding => "Funding",
            Self::EmaDistance => "EMA distance",
        }
    }

    pub fn sort_column(self) -> LiveWatchlistSortColumn {
        match self {
            Self::Price => LiveWatchlistSortColumn::Price,
            Self::Change5m => LiveWatchlistSortColumn::Change5m,
            Self::Change30m => LiveWatchlistSortColumn::Change30m,
            Self::Change1h => LiveWatchlistSortColumn::Change1h,
            Self::Change24h => LiveWatchlistSortColumn::Change24h,
            Self::Funding => LiveWatchlistSortColumn::Funding,
            Self::EmaDistance => LiveWatchlistSortColumn::EmaDistance,
        }
    }

    pub fn width(self) -> f32 {
        match self {
            Self::Price => 70.0,
            Self::EmaDistance => 88.0,
            Self::Change5m | Self::Change30m | Self::Change1h => 50.0,
            Self::Change24h | Self::Funding => 60.0,
        }
    }
}

pub fn default_live_watchlist_columns() -> Vec<LiveWatchlistColumn> {
    LiveWatchlistColumn::ALL
        .into_iter()
        .filter(|column| *column != LiveWatchlistColumn::EmaDistance)
        .collect()
}

pub type WatchlistPresetId = u64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatchlistPresetConfig {
    pub id: WatchlistPresetId,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub symbols: Vec<String>,
}

impl WatchlistPresetConfig {
    pub fn display_name(&self) -> &str {
        let name = self.name.trim();
        if name.is_empty() {
            "Untitled Watchlist"
        } else {
            name
        }
    }
}

impl std::fmt::Display for WatchlistPresetConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "Ascending" => Some(Self::Ascending),
            "Descending" => Some(Self::Descending),
            _ => None,
        }
    }

    fn config_value(self) -> &'static str {
        match self {
            Self::Ascending => "Ascending",
            Self::Descending => "Descending",
        }
    }
}

impl<'de> Deserialize<'de> for SortDirection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_config_value(&value).unwrap_or_else(|| {
            let default = Self::default();
            crate::config::push_config_warning(format!(
                "Unknown sort direction {value:?} in config; using {}",
                default.config_value()
            ));
            default
        }))
    }
}

fn deserialize_visible_columns<'de, D>(
    deserializer: D,
) -> Result<Vec<LiveWatchlistColumn>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<String>::deserialize(deserializer).map(|columns| {
        columns
            .into_iter()
            .filter_map(|value| match LiveWatchlistColumn::from_config_value(&value) {
                Some(column) => Some(column),
                None => {
                    crate::config::push_config_warning(format!(
                        "Unknown live watchlist visible column {value:?} in config; dropping column"
                    ));
                    None
                }
            })
            .collect()
    })
}

fn push_unknown_live_watchlist_sort_value_warning(field: &str, value: &str, fallback: &str) {
    crate::config::push_config_warning(format!(
        "Unknown live watchlist {field} {value:?} in config; using {fallback}"
    ));
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveWatchlistConfig {
    pub id: u64,
    /// Shared named asset list used by this widget. The inline symbols remain
    /// as a portable fallback for old or imported layouts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<WatchlistPresetId>,
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default)]
    pub sort_column: LiveWatchlistSortColumn,
    #[serde(default)]
    pub sort_direction: SortDirection,
    #[serde(
        default = "default_live_watchlist_columns",
        deserialize_with = "deserialize_visible_columns"
    )]
    pub visible_columns: Vec<LiveWatchlistColumn>,
    #[serde(default)]
    pub ema: LiveWatchlistEmaConfig,
}

/// Close-price EMA used by one live watchlist. Candle intervals use API notation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct LiveWatchlistEmaConfig {
    #[serde(deserialize_with = "deserialize_ema_period")]
    pub period: usize,
    #[serde(deserialize_with = "deserialize_ema_timeframe")]
    pub timeframe: String,
}

impl Default for LiveWatchlistEmaConfig {
    fn default() -> Self {
        Self {
            period: 20,
            timeframe: "1h".to_string(),
        }
    }
}

impl LiveWatchlistEmaConfig {
    pub const MAX_PERIOD: usize = 1_000;
    pub const TIMEFRAMES: [&'static str; 14] = [
        "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "8h", "12h", "1d", "3d", "1w", "1M",
    ];

    pub(crate) fn interval(&self) -> crate::timeframe::Timeframe {
        crate::timeframe::Timeframe::from_api_str_opt(&self.timeframe)
            .unwrap_or(crate::timeframe::Timeframe::H1)
    }
}

fn deserialize_ema_period<'de, D: Deserializer<'de>>(deserializer: D) -> Result<usize, D::Error> {
    usize::deserialize(deserializer)
        .map(|period| period.clamp(1, LiveWatchlistEmaConfig::MAX_PERIOD))
}

fn deserialize_ema_timeframe<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    String::deserialize(deserializer).map(|timeframe| {
        if LiveWatchlistEmaConfig::TIMEFRAMES.contains(&timeframe.as_str()) {
            timeframe
        } else {
            LiveWatchlistEmaConfig::default().timeframe
        }
    })
}
