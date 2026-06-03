use crate::account_analytics::{IncomeSnapshot, PortfolioHistory};
use crate::portfolio_state::PnlValueDisplayMode;
use chrono::{Datelike, TimeZone, Utc};

// ---------------------------------------------------------------------------
// Portfolio Selection State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PortfolioScope {
    All,
    Perp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PortfolioWindow {
    Day,
    Week,
    Mtd,
    Month,
    Quarter,
    HalfYear,
    Ytd,
    Year,
    AllTime,
}

impl Default for PortfolioWindow {
    fn default() -> Self {
        Self::Week
    }
}

impl PortfolioWindow {
    pub(crate) fn label(self) -> &'static str {
        match self {
            PortfolioWindow::Day => "1D",
            PortfolioWindow::Week => "1W",
            PortfolioWindow::Mtd => "MTD",
            PortfolioWindow::Month => "1M",
            PortfolioWindow::Quarter => "3M",
            PortfolioWindow::HalfYear => "6M",
            PortfolioWindow::Ytd => "YTD",
            PortfolioWindow::Year => "1Y",
            PortfolioWindow::AllTime => "ALL",
        }
    }

    pub(crate) fn cutoff_ms(self, now_ms: u64) -> Option<u64> {
        const DAY_MS: u64 = 24 * 60 * 60 * 1000;
        match self {
            PortfolioWindow::Day => Some(now_ms.saturating_sub(DAY_MS)),
            PortfolioWindow::Week => Some(now_ms.saturating_sub(7 * DAY_MS)),
            PortfolioWindow::Month => Some(now_ms.saturating_sub(30 * DAY_MS)),
            PortfolioWindow::Quarter => Some(now_ms.saturating_sub(90 * DAY_MS)),
            PortfolioWindow::HalfYear => Some(now_ms.saturating_sub(180 * DAY_MS)),
            PortfolioWindow::Year => Some(now_ms.saturating_sub(365 * DAY_MS)),
            PortfolioWindow::Mtd => {
                let now = Utc
                    .timestamp_millis_opt(i64::try_from(now_ms).ok()?)
                    .single()?;
                let start = Utc
                    .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
                    .single()?;
                u64::try_from(start.timestamp_millis()).ok()
            }
            PortfolioWindow::Ytd => {
                let now = Utc
                    .timestamp_millis_opt(i64::try_from(now_ms).ok()?)
                    .single()?;
                let start = Utc.with_ymd_and_hms(now.year(), 1, 1, 0, 0, 0).single()?;
                u64::try_from(start.timestamp_millis()).ok()
            }
            PortfolioWindow::AllTime => None,
        }
    }
}

pub(crate) const PORTFOLIO_WINDOWS: &[PortfolioWindow] = &[
    PortfolioWindow::Day,
    PortfolioWindow::Week,
    PortfolioWindow::Mtd,
    PortfolioWindow::Month,
    PortfolioWindow::Quarter,
    PortfolioWindow::HalfYear,
    PortfolioWindow::Ytd,
    PortfolioWindow::Year,
    PortfolioWindow::AllTime,
];

#[derive(Debug, Clone)]
pub(crate) struct PortfolioState {
    pub(crate) loading: bool,
    pub(crate) scope: PortfolioScope,
    pub(crate) window: PortfolioWindow,
    pub(crate) pnl_value_display_mode: PnlValueDisplayMode,
    pub(crate) data: Option<PortfolioHistory>,
    pub(crate) last_error: Option<String>,
}

impl Default for PortfolioState {
    fn default() -> Self {
        Self {
            loading: false,
            scope: PortfolioScope::All,
            window: PortfolioWindow::Week,
            pnl_value_display_mode: PnlValueDisplayMode::Usd,
            data: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct IncomeState {
    pub(crate) loading: bool,
    pub(crate) data: Option<IncomeSnapshot>,
    pub(crate) last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp_ms(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> u64 {
        let datetime = Utc
            .with_ymd_and_hms(year, month, day, hour, min, sec)
            .single()
            .expect("test timestamp should be a valid UTC instant");
        u64::try_from(datetime.timestamp_millis()).expect("test timestamp should be positive")
    }

    #[test]
    fn mtd_cutoff_starts_at_current_calendar_month() {
        let now_ms = timestamp_ms(2026, 6, 15, 14, 30, 12);
        let expected = timestamp_ms(2026, 6, 1, 0, 0, 0);

        assert_eq!(PortfolioWindow::Mtd.cutoff_ms(now_ms), Some(expected));
    }

    #[test]
    fn mtd_cutoff_handles_first_day_of_month() {
        let now_ms = timestamp_ms(2026, 6, 1, 0, 0, 0);

        assert_eq!(PortfolioWindow::Mtd.cutoff_ms(now_ms), Some(now_ms));
    }

    #[test]
    fn portfolio_windows_include_mtd_before_rolling_month() {
        let labels: Vec<_> = PORTFOLIO_WINDOWS
            .iter()
            .map(|window| window.label())
            .collect();

        assert_eq!(
            labels,
            vec!["1D", "1W", "MTD", "1M", "3M", "6M", "YTD", "1Y", "ALL"]
        );
    }
}
