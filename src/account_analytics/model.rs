use serde::Deserialize;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Account Analytics Types
// ---------------------------------------------------------------------------

/// Borrow-lend reserve parameters for a token.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowLendReserveState {
    pub borrow_yearly_rate: String,
    pub supply_yearly_rate: String,
    pub oracle_px: String,
}

/// Borrow/lend side state for a token position.
#[derive(Debug, Clone, Deserialize)]
pub struct BorrowLendSideState {
    pub value: String,
}

/// User borrow-lend state response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowLendUserState {
    pub token_to_state: Vec<(u32, BorrowLendTokenState)>,
    pub health: String,
    pub health_factor: Option<String>,
}

/// Borrow/lend state for one token.
#[derive(Debug, Clone, Deserialize)]
pub struct BorrowLendTokenState {
    pub borrow: BorrowLendSideState,
    pub supply: BorrowLendSideState,
}

/// Interest accrual row from `userBorrowLendInterest`.
#[derive(Debug, Clone, Deserialize)]
pub struct BorrowLendInterestEntry {
    pub time: u64,
    pub token: String,
    pub borrow: String,
    pub supply: String,
    #[serde(rename = "nSamples")]
    pub n_samples: Option<u32>,
}

/// Per-token contribution to projected net interest.
#[derive(Debug, Clone)]
pub struct IncomeTokenRow {
    pub token: u32,
    pub token_label: String,
    pub supply_usd: f64,
    pub borrow_usd: f64,
    pub supply_rate: f64,
    pub net_yearly_usd: f64,
}

/// Computed income snapshot for portfolio-margin borrow/lend accounts.
#[derive(Debug, Clone)]
pub struct IncomeSnapshot {
    pub earned_total: f64,
    pub earned_24h: f64,
    pub earned_7d: f64,
    pub earned_30d: f64,
    pub net_yearly_projection: f64,
    pub current_supply_usd: f64,
    pub current_borrow_usd: f64,
    pub health: String,
    pub health_factor: Option<String>,
    pub token_rows: Vec<IncomeTokenRow>,
    pub recent_hourly_payments: Vec<IncomeHourlyPayment>,
    pub invalid_token_rows: usize,
    pub invalid_interest_rows: usize,
}

#[derive(Debug, Clone)]
pub struct IncomeHourlyPayment {
    pub time: u64,
    pub token_label: String,
    pub supply: f64,
    pub borrow: f64,
    pub net: f64,
}

/// Parsed portfolio history for all supported windows.
#[derive(Debug, Clone, Default)]
pub struct PortfolioHistory {
    pub buckets: HashMap<String, PortfolioBucket>,
}

/// A single portfolio history bucket (e.g. `day`, `perpWeek`).
#[derive(Debug, Clone, Default)]
pub struct PortfolioBucket {
    pub account_value_history: Vec<(u64, f64)>,
    pub pnl_history: Vec<(u64, f64)>,
    pub vlm: Option<f64>,
    pub skipped_invalid_points: usize,
    pub invalid_vlm: bool,
}
