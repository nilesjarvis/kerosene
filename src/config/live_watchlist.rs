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
}

impl LiveWatchlistColumn {
    pub const ALL: [Self; 6] = [
        Self::Price,
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
        }
    }

    pub fn width(self) -> f32 {
        match self {
            Self::Price => 70.0,
            Self::Change5m | Self::Change30m | Self::Change1h => 50.0,
            Self::Change24h | Self::Funding => 60.0,
        }
    }
}

pub fn default_live_watchlist_columns() -> Vec<LiveWatchlistColumn> {
    LiveWatchlistColumn::ALL.to_vec()
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
}
