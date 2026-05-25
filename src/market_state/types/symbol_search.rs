// ---------------------------------------------------------------------------
// Symbol Search Settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SymbolSearchSortMode {
    #[default]
    Relevance,
    Volume24h,
    Alphabetical,
    Exchange,
}

impl SymbolSearchSortMode {
    pub(crate) const ALL: [Self; 4] = [
        Self::Relevance,
        Self::Volume24h,
        Self::Alphabetical,
        Self::Exchange,
    ];

    pub(crate) fn from_config_str(value: &str) -> Self {
        match value {
            "24h_volume" => Self::Volume24h,
            "alphabetical" => Self::Alphabetical,
            "exchange" => Self::Exchange,
            _ => Self::Relevance,
        }
    }

    pub(crate) fn config_value(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Volume24h => "24h_volume",
            Self::Alphabetical => "alphabetical",
            Self::Exchange => "exchange",
        }
    }
}

impl std::fmt::Display for SymbolSearchSortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Relevance => write!(f, "Relevance"),
            Self::Volume24h => write!(f, "24h Vol"),
            Self::Alphabetical => write!(f, "A-Z"),
            Self::Exchange => write!(f, "Exchange"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SymbolSearchMarketFilter {
    #[default]
    All,
    NativePerps,
    Spot,
    Hip3,
    Outcomes,
}

impl SymbolSearchMarketFilter {
    pub(crate) const ALL: [Self; 5] = [
        Self::All,
        Self::NativePerps,
        Self::Spot,
        Self::Hip3,
        Self::Outcomes,
    ];
}

impl std::fmt::Display for SymbolSearchMarketFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "All Markets"),
            Self::NativePerps => write!(f, "Native Perps"),
            Self::Spot => write!(f, "Spot"),
            Self::Hip3 => write!(f, "HIP-3"),
            Self::Outcomes => write!(f, "Outcomes"),
        }
    }
}

pub(crate) const SYMBOL_SEARCH_ALL_HIP3_DEXES: &str = "All HIP-3";
