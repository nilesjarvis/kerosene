mod history;

use crate::account_analytics::PortfolioBucket;
use crate::app_state::TradingTerminal;

use self::history::{
    apply_cutoff_with_baseline, compute_daily_percent_rows_from_cumulative,
    compute_daily_pnl_rows_from_cumulative, compute_percent_performance_series,
};
use super::{PortfolioScope, PortfolioWindow};

// ---------------------------------------------------------------------------
// Portfolio Data Selection
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn portfolio_bucket_by_key(&self, key: &str) -> Option<&PortfolioBucket> {
        let _theme = self.theme();
        self.portfolio
            .data
            .as_ref()
            .and_then(|d| d.buckets.get(key))
    }

    pub(crate) fn portfolio_alltime_bucket(&self) -> Option<&PortfolioBucket> {
        let _theme = self.theme();
        let key = match self.portfolio.scope {
            PortfolioScope::All => "allTime",
            PortfolioScope::Perp => "perpAllTime",
        };
        self.portfolio_bucket_by_key(key)
    }

    pub(crate) fn portfolio_window_bucket(&self) -> Option<&PortfolioBucket> {
        let _theme = self.theme();
        let direct_key = match (self.portfolio.scope, self.portfolio.window) {
            (PortfolioScope::All, PortfolioWindow::Day) => Some("day"),
            (PortfolioScope::All, PortfolioWindow::Week) => Some("week"),
            (PortfolioScope::All, PortfolioWindow::Month) => Some("month"),
            (PortfolioScope::Perp, PortfolioWindow::Day) => Some("perpDay"),
            (PortfolioScope::Perp, PortfolioWindow::Week) => Some("perpWeek"),
            (PortfolioScope::Perp, PortfolioWindow::Month) => Some("perpMonth"),
            _ => None,
        };

        direct_key
            .and_then(|k| self.portfolio_bucket_by_key(k))
            .or_else(|| self.portfolio_alltime_bucket())
    }

    pub(crate) fn daily_source_portfolio_bucket(&self) -> Option<&PortfolioBucket> {
        let _theme = self.theme();
        let primary = match self.portfolio.scope {
            PortfolioScope::All => "week",
            PortfolioScope::Perp => "perpWeek",
        };
        let fallback = match self.portfolio.scope {
            PortfolioScope::All => "month",
            PortfolioScope::Perp => "perpMonth",
        };
        self.portfolio_bucket_by_key(primary)
            .or_else(|| self.portfolio_bucket_by_key(fallback))
            .or_else(|| self.portfolio_alltime_bucket())
    }

    pub(crate) fn apply_cutoff_with_baseline(
        points: &[(u64, f64)],
        cutoff: u64,
    ) -> Vec<(u64, f64)> {
        apply_cutoff_with_baseline(points, cutoff)
    }

    pub(crate) fn selected_portfolio_points(&self) -> Vec<(u64, f64)> {
        let _theme = self.theme();
        let points = self
            .portfolio_window_bucket()
            .map(|b| b.pnl_history.clone())
            .unwrap_or_default();
        self.apply_selected_portfolio_window(points)
    }

    pub(crate) fn selected_portfolio_account_value_points(&self) -> Vec<(u64, f64)> {
        let _theme = self.theme();
        let points = self
            .portfolio_window_bucket()
            .map(|b| b.account_value_history.clone())
            .unwrap_or_default();
        self.apply_selected_portfolio_window(points)
    }

    pub(crate) fn selected_portfolio_performance_points(&self) -> Vec<(u64, f64)> {
        let pnl_points = self.selected_portfolio_points();
        let account_value_points = self.selected_portfolio_account_value_points();
        compute_percent_performance_series(&pnl_points, &account_value_points)
    }

    fn apply_selected_portfolio_window(&self, points: Vec<(u64, f64)>) -> Vec<(u64, f64)> {
        if points.is_empty() {
            return points;
        }
        let now_ms = Self::now_ms();
        match self.portfolio.window {
            PortfolioWindow::Day | PortfolioWindow::Week | PortfolioWindow::Month => points,
            _ => {
                if let Some(cutoff) = self.portfolio.window.cutoff_ms(now_ms) {
                    Self::apply_cutoff_with_baseline(&points, cutoff)
                } else {
                    points
                }
            }
        }
    }

    pub(crate) fn compute_daily_pnl_rows_from_cumulative(
        points: &[(u64, f64)],
        max_days: usize,
    ) -> Vec<(String, f64)> {
        compute_daily_pnl_rows_from_cumulative(points, max_days)
    }

    pub(crate) fn compute_daily_percent_rows_from_cumulative(
        pnl_points: &[(u64, f64)],
        account_value_points: &[(u64, f64)],
        max_days: usize,
    ) -> Vec<(String, f64)> {
        compute_daily_percent_rows_from_cumulative(pnl_points, account_value_points, max_days)
    }
}
