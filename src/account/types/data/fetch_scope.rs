// ---------------------------------------------------------------------------
// Account Data Fetch Scope
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountDataFetchScope {
    AllMarkets { hip3_dexes: Vec<String> },
    Hip3Dex { dex: String },
}

impl Default for AccountDataFetchScope {
    fn default() -> Self {
        Self::all_markets(crate::account::HIP3_DEXES.iter().copied())
    }
}

impl AccountDataFetchScope {
    pub fn all_markets(dexes: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let mut hip3_dexes = dexes
            .into_iter()
            .filter_map(|dex| normalized_hip3_dex(dex.as_ref()))
            .collect::<Vec<_>>();
        hip3_dexes.sort();
        hip3_dexes.dedup();
        if hip3_dexes.is_empty() {
            hip3_dexes = crate::account::HIP3_DEXES
                .iter()
                .map(|dex| (*dex).to_string())
                .collect();
        }
        Self::AllMarkets { hip3_dexes }
    }

    pub fn hip3_dex(dex: impl Into<String>) -> Self {
        normalized_hip3_dex(&dex.into())
            .map(|dex| Self::Hip3Dex { dex })
            .unwrap_or_default()
    }

    pub fn selected_hip3_dex(&self) -> Option<&str> {
        match self {
            Self::AllMarkets { .. } => None,
            Self::Hip3Dex { dex } => Some(dex.as_str()),
        }
    }

    pub fn hip3_dexes(&self, all_dexes: &[&str]) -> Vec<String> {
        match self {
            Self::AllMarkets { hip3_dexes } => {
                if hip3_dexes.is_empty() {
                    all_dexes.iter().map(|dex| (*dex).to_string()).collect()
                } else {
                    hip3_dexes.clone()
                }
            }
            Self::Hip3Dex { dex } => vec![dex.clone()],
        }
    }

    pub fn fetches_main_open_orders(&self) -> bool {
        matches!(self, Self::AllMarkets { .. })
    }

    pub fn estimated_info_weight(&self) -> u32 {
        // Hyperliquid currently weights clearinghouseState and spotClearinghouseState at 2.
        // User fills/funding have return-size adders, so use a conservative base estimate.
        let main_clearinghouse = 2;
        let spot = 2;
        let account_abstraction = 20;
        let fills = 40;
        let funding = 40;
        let fees = 20;
        let main_orders = if self.fetches_main_open_orders() {
            20
        } else {
            0
        };
        let hip3_per_dex = 2 + 20;
        main_clearinghouse
            + spot
            + account_abstraction
            + fills
            + funding
            + fees
            + main_orders
            + hip3_per_dex * self.hip3_dex_count()
    }

    pub fn automatic_refresh_interval_secs(&self) -> u64 {
        const ACCOUNT_REFRESH_WEIGHT_BUDGET_PER_MIN: u32 = 240;
        let secs = (self.estimated_info_weight() as u64 * 60)
            .div_ceil(ACCOUNT_REFRESH_WEIGHT_BUDGET_PER_MIN as u64);
        secs.clamp(45, 180)
    }

    fn hip3_dex_count(&self) -> u32 {
        match self {
            Self::AllMarkets { hip3_dexes } => hip3_dexes.len() as u32,
            Self::Hip3Dex { .. } => 1,
        }
    }
}

fn normalized_hip3_dex(dex: &str) -> Option<String> {
    let dex = dex.trim().to_ascii_lowercase();
    (!dex.is_empty()).then_some(dex)
}
