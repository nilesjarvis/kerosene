use serde::Deserialize;
use std::{collections::HashMap, fmt};

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
#[derive(Clone, Deserialize)]
pub struct BorrowLendSideState {
    pub value: String,
}

impl fmt::Debug for BorrowLendSideState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowLendSideState")
            .field("value", &"<redacted>")
            .finish()
    }
}

/// User borrow-lend state response.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowLendUserState {
    pub token_to_state: Vec<(u32, BorrowLendTokenState)>,
    pub health: String,
    pub health_factor: Option<String>,
}

impl fmt::Debug for BorrowLendUserState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowLendUserState")
            .field("token_state_count", &self.token_to_state.len())
            .field("health", &"<redacted>")
            .field("has_health_factor", &self.health_factor.is_some())
            .finish()
    }
}

/// Borrow/lend state for one token.
#[derive(Clone, Deserialize)]
pub struct BorrowLendTokenState {
    pub borrow: BorrowLendSideState,
    pub supply: BorrowLendSideState,
}

impl fmt::Debug for BorrowLendTokenState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowLendTokenState")
            .field("borrow", &"<redacted>")
            .field("supply", &"<redacted>")
            .finish()
    }
}

/// Interest accrual row from `userBorrowLendInterest`.
#[derive(Clone, Deserialize)]
pub struct BorrowLendInterestEntry {
    pub time: u64,
    pub token: String,
    pub borrow: String,
    pub supply: String,
    #[serde(rename = "nSamples")]
    pub n_samples: Option<u32>,
}

impl fmt::Debug for BorrowLendInterestEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowLendInterestEntry")
            .field("time", &"<redacted>")
            .field("token", &"<redacted>")
            .field("borrow", &"<redacted>")
            .field("supply", &"<redacted>")
            .field("n_samples", &self.n_samples)
            .finish()
    }
}

/// Per-token contribution to projected net interest.
#[derive(Clone)]
pub struct IncomeTokenRow {
    pub token: u32,
    pub token_label: String,
    pub supply_usd: f64,
    pub borrow_usd: f64,
    pub supply_rate: f64,
    pub net_yearly_usd: f64,
}

impl fmt::Debug for IncomeTokenRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IncomeTokenRow")
            .field("token", &"<redacted>")
            .field("token_label", &"<redacted>")
            .field("supply_usd", &"<redacted>")
            .field("borrow_usd", &"<redacted>")
            .field("supply_rate", &"<redacted>")
            .field("net_yearly_usd", &"<redacted>")
            .finish()
    }
}

/// Computed income snapshot for portfolio-margin borrow/lend accounts.
#[derive(Clone)]
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

impl fmt::Debug for IncomeSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IncomeSnapshot")
            .field("earned_total", &"<redacted>")
            .field("earned_24h", &"<redacted>")
            .field("earned_7d", &"<redacted>")
            .field("earned_30d", &"<redacted>")
            .field("net_yearly_projection", &"<redacted>")
            .field("current_supply_usd", &"<redacted>")
            .field("current_borrow_usd", &"<redacted>")
            .field("health", &"<redacted>")
            .field("has_health_factor", &self.health_factor.is_some())
            .field("token_rows_count", &self.token_rows.len())
            .field(
                "recent_hourly_payments_count",
                &self.recent_hourly_payments.len(),
            )
            .field("invalid_token_rows", &self.invalid_token_rows)
            .field("invalid_interest_rows", &self.invalid_interest_rows)
            .finish()
    }
}

#[derive(Clone)]
pub struct IncomeHourlyPayment {
    pub time: u64,
    pub token_label: String,
    pub supply: f64,
    pub borrow: f64,
    pub net: f64,
}

impl fmt::Debug for IncomeHourlyPayment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IncomeHourlyPayment")
            .field("time", &"<redacted>")
            .field("token_label", &"<redacted>")
            .field("supply", &"<redacted>")
            .field("borrow", &"<redacted>")
            .field("net", &"<redacted>")
            .finish()
    }
}

/// Parsed portfolio history for all supported windows.
#[derive(Clone, Default)]
pub struct PortfolioHistory {
    pub buckets: HashMap<String, PortfolioBucket>,
}

impl fmt::Debug for PortfolioHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PortfolioHistory")
            .field("buckets_count", &self.buckets.len())
            .finish()
    }
}

/// A single portfolio history bucket (e.g. `day`, `perpWeek`).
#[derive(Clone, Default)]
pub struct PortfolioBucket {
    pub account_value_history: Vec<(u64, f64)>,
    pub pnl_history: Vec<(u64, f64)>,
    pub vlm: Option<f64>,
    pub skipped_invalid_points: usize,
    pub invalid_vlm: bool,
}

impl fmt::Debug for PortfolioBucket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PortfolioBucket")
            .field(
                "account_value_history_count",
                &self.account_value_history.len(),
            )
            .field("pnl_history_count", &self.pnl_history.len())
            .field("has_vlm", &self.vlm.is_some())
            .field("skipped_invalid_points", &self.skipped_invalid_points)
            .field("invalid_vlm", &self.invalid_vlm)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BorrowLendInterestEntry, BorrowLendSideState, BorrowLendTokenState, BorrowLendUserState,
        IncomeHourlyPayment, IncomeSnapshot, IncomeTokenRow, PortfolioBucket, PortfolioHistory,
    };
    use std::collections::HashMap;

    #[test]
    fn private_account_analytics_debug_is_structural_and_exact_values_survive() {
        const ACCOUNT_VALUE: f64 = 98_765.432_1;
        const PNL_VALUE: f64 = -12_345.678_9;
        const VOLUME: f64 = 45_678.912_3;
        let user = BorrowLendUserState {
            token_to_state: vec![(
                7,
                BorrowLendTokenState {
                    borrow: BorrowLendSideState {
                        value: "private-borrow-sentinel".to_string(),
                    },
                    supply: BorrowLendSideState {
                        value: "private-supply-sentinel".to_string(),
                    },
                },
            )],
            health: "private-health-sentinel".to_string(),
            health_factor: Some("private-health-factor-sentinel".to_string()),
        };
        let interest = BorrowLendInterestEntry {
            time: 9_876_543_210,
            token: "private-interest-token-sentinel".to_string(),
            borrow: "private-interest-borrow-sentinel".to_string(),
            supply: "private-interest-supply-sentinel".to_string(),
            n_samples: Some(3),
        };
        let token_row = IncomeTokenRow {
            token: 777,
            token_label: "private-income-token-sentinel".to_string(),
            supply_usd: ACCOUNT_VALUE,
            borrow_usd: VOLUME,
            supply_rate: 0.123_456_789,
            net_yearly_usd: PNL_VALUE,
        };
        let payment = IncomeHourlyPayment {
            time: 9_876_543_211,
            token_label: "private-payment-token-sentinel".to_string(),
            supply: ACCOUNT_VALUE,
            borrow: VOLUME,
            net: PNL_VALUE,
        };
        let snapshot = IncomeSnapshot {
            earned_total: ACCOUNT_VALUE,
            earned_24h: PNL_VALUE,
            earned_7d: VOLUME,
            earned_30d: ACCOUNT_VALUE,
            net_yearly_projection: PNL_VALUE,
            current_supply_usd: ACCOUNT_VALUE,
            current_borrow_usd: VOLUME,
            health: "private-snapshot-health-sentinel".to_string(),
            health_factor: Some("private-snapshot-factor-sentinel".to_string()),
            token_rows: vec![token_row],
            recent_hourly_payments: vec![payment],
            invalid_token_rows: 1,
            invalid_interest_rows: 2,
        };
        let history = PortfolioHistory {
            buckets: HashMap::from([(
                "private-bucket-key-sentinel".to_string(),
                PortfolioBucket {
                    account_value_history: vec![(9_876_543_212, ACCOUNT_VALUE)],
                    pnl_history: vec![(9_876_543_213, PNL_VALUE)],
                    vlm: Some(VOLUME),
                    skipped_invalid_points: 4,
                    invalid_vlm: false,
                },
            )]),
        };

        let rendered = format!(
            "{user:?} {:?} {interest:?} {snapshot:?} {:?} {:?} {history:?} {:?}",
            &user.token_to_state[0].1,
            &snapshot.token_rows[0],
            &snapshot.recent_hourly_payments[0],
            &history.buckets["private-bucket-key-sentinel"],
        );

        assert!(rendered.contains("token_state_count: 1"), "{rendered}");
        assert!(rendered.contains("token_rows_count: 1"), "{rendered}");
        assert!(rendered.contains("buckets_count: 1"), "{rendered}");
        for sensitive in [
            "private-borrow-sentinel",
            "private-supply-sentinel",
            "private-health-sentinel",
            "private-health-factor-sentinel",
            "private-interest-token-sentinel",
            "private-interest-borrow-sentinel",
            "private-interest-supply-sentinel",
            "private-income-token-sentinel",
            "private-payment-token-sentinel",
            "private-snapshot-health-sentinel",
            "private-snapshot-factor-sentinel",
            "private-bucket-key-sentinel",
        ] {
            assert!(!rendered.contains(sensitive), "{rendered}");
        }
        for value in [ACCOUNT_VALUE, PNL_VALUE, VOLUME] {
            assert!(!rendered.contains(&format!("{value:?}")), "{rendered}");
        }
        assert_eq!(
            user.token_to_state[0].1.borrow.value,
            "private-borrow-sentinel"
        );
        assert_eq!(interest.time, 9_876_543_210);
        assert_eq!(snapshot.earned_total.to_bits(), ACCOUNT_VALUE.to_bits());
        assert_eq!(
            history.buckets["private-bucket-key-sentinel"].pnl_history[0]
                .1
                .to_bits(),
            PNL_VALUE.to_bits()
        );
    }
}
