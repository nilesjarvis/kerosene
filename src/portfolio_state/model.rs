use super::refresh::AnalyticsRefreshState;
use crate::account_analytics::{IncomeSnapshot, PortfolioHistory};
use crate::app_time::DisplayTimeZone;
use crate::portfolio_state::PnlValueDisplayMode;
#[cfg(test)]
use chrono::Utc;
use chrono::{Datelike, TimeZone};

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

    pub(crate) fn cutoff_ms(self, now_ms: u64, zone: DisplayTimeZone) -> Option<u64> {
        const DAY_MS: u64 = 24 * 60 * 60 * 1000;
        match self {
            PortfolioWindow::Day => Some(now_ms.saturating_sub(DAY_MS)),
            PortfolioWindow::Week => Some(now_ms.saturating_sub(7 * DAY_MS)),
            PortfolioWindow::Month => Some(now_ms.saturating_sub(30 * DAY_MS)),
            PortfolioWindow::Quarter => Some(now_ms.saturating_sub(90 * DAY_MS)),
            PortfolioWindow::HalfYear => Some(now_ms.saturating_sub(180 * DAY_MS)),
            PortfolioWindow::Year => Some(now_ms.saturating_sub(365 * DAY_MS)),
            PortfolioWindow::Mtd | PortfolioWindow::Ytd => {
                let now = zone.datetime(now_ms)?;
                let month = if self == Self::Ytd { 1 } else { now.month() };
                let start =
                    chrono::NaiveDate::from_ymd_opt(now.year(), month, 1)?.and_hms_opt(0, 0, 0)?;
                let start = zone
                    .0
                    .from_local_datetime(&start)
                    .earliest()
                    .or_else(|| chrono_tz::GapInfo::new(&start, &zone.0)?.end)?;
                Some(start.timestamp_millis().max(0) as u64)
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

        assert_eq!(
            PortfolioWindow::Mtd.cutoff_ms(now_ms, Default::default()),
            Some(expected)
        );
    }

    #[test]
    fn mtd_cutoff_handles_first_day_of_month() {
        let now_ms = timestamp_ms(2026, 6, 1, 0, 0, 0);

        assert_eq!(
            PortfolioWindow::Mtd.cutoff_ms(now_ms, Default::default()),
            Some(now_ms)
        );
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
    #[test]
    fn calendar_cutoffs_follow_the_selected_city_across_new_year() {
        let now: chrono::DateTime<Utc> = "2026-01-01T02:00:00Z".parse().expect("valid instant");
        let zone = DisplayTimeZone(chrono_tz::America::New_York);
        for (window, expected) in [
            (PortfolioWindow::Mtd, "2025-12-01T05:00:00Z"),
            (PortfolioWindow::Ytd, "2025-01-01T05:00:00Z"),
        ] {
            let expected: chrono::DateTime<Utc> = expected.parse().expect("valid midnight");
            assert_eq!(
                window.cutoff_ms(now.timestamp_millis() as u64, zone),
                Some(expected.timestamp_millis() as u64)
            );
        }
    }
}
