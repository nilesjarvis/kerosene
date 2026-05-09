#[derive(Debug, Clone)]
pub(in crate::assistant) struct DrawdownDcaResult {
    pub symbol: String,
    pub interval: String,
    pub lookback_days: u32,
    pub drawdown_pct: f64,
    pub tranche_usd: f64,
    pub entries: usize,
    pub invested_usd: f64,
    pub units: f64,
    pub end_price: f64,
    pub ending_value_usd: f64,
    pub pnl_usd: f64,
    pub roi_pct: f64,
}

#[derive(Debug, Clone)]
pub(in crate::assistant) struct PriceLookupResult {
    pub symbol: String,
    pub interval: String,
    pub price: f64,
    pub candle_time: Option<u64>,
    pub source: String,
}
