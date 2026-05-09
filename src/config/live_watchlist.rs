use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
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
    #[serde(default = "default_live_watchlist_columns")]
    pub visible_columns: Vec<LiveWatchlistColumn>,
}
