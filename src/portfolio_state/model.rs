use super::refresh::AnalyticsRefreshState;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PortfolioWindow {
    Day,
    #[default]
    Week,
    Mtd,
    Month,
    Quarter,
    HalfYear,
    Ytd,
    Year,
    AllTime,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum IncomePaneView {
    #[default]
    Overview,
    Tokens,
    Payments,
}

impl IncomePaneView {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Tokens => "Tokens",
            Self::Payments => "Payments",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PortfolioState {
    pub(crate) refresh: AnalyticsRefreshState,
    pub(crate) scope: PortfolioScope,
    pub(crate) window: PortfolioWindow,
    pub(crate) pnl_value_display_mode: PnlValueDisplayMode,
    pub(crate) data: Option<PortfolioHistory>,
    pub(crate) last_error: Option<String>,
}

impl Default for PortfolioState {
    fn default() -> Self {
        Self {
            refresh: AnalyticsRefreshState::default(),
            scope: PortfolioScope::All,
            // Spec default: the all-time window is active on first load so the
            // hero shows the headline lifetime PnL.
            window: PortfolioWindow::AllTime,
            pnl_value_display_mode: PnlValueDisplayMode::Usd,
            data: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct IncomeState {
    pub(crate) refresh: AnalyticsRefreshState,
    pub(crate) view: IncomePaneView,
    pub(crate) data: Option<IncomeSnapshot>,
    pub(crate) last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_refresh_lifecycle(mut state: AnalyticsRefreshState) {
        assert!(!state.loading);
        assert_eq!(state.request_id, 0);
        assert!(!state.take_followup());

        let first = state.begin();
        assert_eq!(first, 1);
        state.queue_followup();
        state.queue_followup();
        assert!(!state.finish(first - 1));
        assert!(state.loading);
        assert_eq!(state.request_id, first);
        assert!(state.followup_pending);

        assert!(state.finish(first));
        assert!(!state.loading);
        assert_eq!(state.request_id, first + 1);
        assert!(!state.finish(first));
        assert!(state.take_followup());
        assert!(!state.take_followup());

        state.queue_followup();
        let second = state.begin();
        assert!(state.followup_pending);
        state.invalidate();
        assert!(!state.loading);
        assert!(!state.take_followup());
        assert_eq!(state.request_id, second + 1);
        assert!(!state.finish(second));

        // The counter intentionally saturates instead of wrapping.
        state.request_id = u64::MAX - 1;
        assert_eq!(state.begin(), u64::MAX);
        assert!(state.finish(u64::MAX));
        assert_eq!(state.request_id, u64::MAX);
        state.queue_followup();
        state.invalidate();
        assert_eq!(state.request_id, u64::MAX);
        assert!(!state.loading);
        assert!(!state.take_followup());
        assert!(!state.finish(u64::MAX - 1));
    }

    #[test]
    fn portfolio_refresh_preserves_completion_followup_and_invalidation_rules() {
        assert_refresh_lifecycle(PortfolioState::default().refresh);
    }

    #[test]
    fn income_refresh_preserves_completion_followup_and_invalidation_rules() {
        assert_refresh_lifecycle(IncomeState::default().refresh);
    }

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
