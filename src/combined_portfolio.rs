use crate::account_analytics::{PortfolioBucket, PortfolioHistory};
use crate::config::{CombinedPortfolioConfig, TrackedWalletConfig};
use crate::portfolio_state::{PortfolioScope, PortfolioWindow};
use crate::wallet_state::address_book::normalize_wallet_address_value;

use iced::window;
use std::collections::{BTreeSet, HashSet};

pub(crate) const DEFAULT_COMBINED_PORTFOLIO_WIDTH: f32 = 1180.0;
pub(crate) const DEFAULT_COMBINED_PORTFOLIO_HEIGHT: f32 = 760.0;

// ---------------------------------------------------------------------------
// Combined portfolio state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct CombinedPortfolioWallet {
    pub(crate) address: String,
    pub(crate) label: String,
    pub(crate) loading: bool,
    pub(crate) request_id: u64,
    pub(crate) history: Option<PortfolioHistory>,
    pub(crate) error: Option<String>,
    pub(crate) last_updated_ms: Option<u64>,
}

impl CombinedPortfolioWallet {
    fn new(address: String, label: String) -> Self {
        Self {
            address,
            label,
            loading: false,
            request_id: 0,
            history: None,
            error: None,
            last_updated_ms: None,
        }
    }
}

pub(crate) struct CombinedPortfolioState {
    pub(crate) window_id: Option<window::Id>,
    pub(crate) open: bool,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) x: Option<f32>,
    pub(crate) y: Option<f32>,
    pub(crate) add_address_input: String,
    pub(crate) add_label_input: String,
    pub(crate) wallets: Vec<CombinedPortfolioWallet>,
    pub(crate) scope: PortfolioScope,
    pub(crate) window: PortfolioWindow,
    next_request_id: u64,
}

impl CombinedPortfolioState {
    pub(crate) fn from_config(config: &CombinedPortfolioConfig) -> Self {
        let mut seen = HashSet::new();
        let wallets = config
            .wallets
            .iter()
            .filter_map(|wallet| {
                let address = normalize_wallet_address_value(&wallet.address)?;
                seen.insert(address.clone())
                    .then(|| CombinedPortfolioWallet::new(address, wallet.label.trim().to_string()))
            })
            .collect();

        Self {
            window_id: None,
            open: config.open,
            width: valid_dimension(config.width, DEFAULT_COMBINED_PORTFOLIO_WIDTH),
            height: valid_dimension(config.height, DEFAULT_COMBINED_PORTFOLIO_HEIGHT),
            x: config.x.filter(|value| value.is_finite()),
            y: config.y.filter(|value| value.is_finite()),
            add_address_input: String::new(),
            add_label_input: String::new(),
            wallets,
            scope: PortfolioScope::All,
            window: PortfolioWindow::AllTime,
            next_request_id: 0,
        }
    }

    pub(crate) fn to_config(&self) -> CombinedPortfolioConfig {
        CombinedPortfolioConfig {
            wallets: self
                .wallets
                .iter()
                .map(|wallet| TrackedWalletConfig {
                    address: wallet.address.clone(),
                    label: wallet.label.clone(),
                })
                .collect(),
            open: self.open,
            width: self.width,
            height: self.height,
            x: self.x,
            y: self.y,
        }
    }

    pub(crate) fn begin_wallet_refresh(&mut self, address: &str) -> Option<u64> {
        let wallet = self
            .wallets
            .iter_mut()
            .find(|wallet| wallet.address == address)?;
        self.next_request_id = self.next_request_id.saturating_add(1);
        wallet.request_id = self.next_request_id;
        wallet.loading = true;
        wallet.error = None;
        Some(wallet.request_id)
    }

    pub(crate) fn wallet_mut_for_result(
        &mut self,
        address: &str,
        request_id: u64,
    ) -> Option<&mut CombinedPortfolioWallet> {
        self.wallets
            .iter_mut()
            .find(|wallet| wallet.address == address && wallet.request_id == request_id)
    }
}

impl Default for CombinedPortfolioState {
    fn default() -> Self {
        Self::from_config(&CombinedPortfolioConfig::default())
    }
}

fn valid_dimension(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

// ---------------------------------------------------------------------------
// History selection and aggregation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct CombinedPortfolioAggregate {
    pub(crate) points: Vec<(u64, f64)>,
    pub(crate) total_pnl: Option<f64>,
    pub(crate) account_value: Option<f64>,
    pub(crate) loaded_wallets: usize,
    pub(crate) profitable_wallets: usize,
}

pub(crate) fn aggregate_portfolios(
    histories: &[&PortfolioHistory],
    scope: PortfolioScope,
    window: PortfolioWindow,
    now_ms: u64,
) -> CombinedPortfolioAggregate {
    let series = histories
        .iter()
        .map(|history| selected_pnl_points(history, scope, window, now_ms))
        .filter(|points| !points.is_empty())
        .collect::<Vec<_>>();
    let total_pnl_values = series
        .iter()
        .filter_map(|points| portfolio_total_pnl(points))
        .collect::<Vec<_>>();
    let total_pnl = (!total_pnl_values.is_empty())
        .then(|| total_pnl_values.iter().sum::<f64>())
        .filter(|value| value.is_finite());
    let profitable_wallets = total_pnl_values
        .iter()
        .filter(|value| **value > 0.0)
        .count();

    let account_values = histories
        .iter()
        .filter_map(|history| latest_account_value(history, scope))
        .collect::<Vec<_>>();
    let account_value = (!account_values.is_empty())
        .then(|| account_values.iter().sum::<f64>())
        .filter(|value| value.is_finite());

    CombinedPortfolioAggregate {
        points: aggregate_relative_series(&series),
        total_pnl,
        account_value,
        loaded_wallets: histories.len(),
        profitable_wallets,
    }
}

pub(crate) fn wallet_period_pnl(
    history: &PortfolioHistory,
    scope: PortfolioScope,
    window: PortfolioWindow,
    now_ms: u64,
) -> Option<f64> {
    portfolio_total_pnl(&selected_pnl_points(history, scope, window, now_ms))
}

pub(crate) fn latest_account_value(
    history: &PortfolioHistory,
    scope: PortfolioScope,
) -> Option<f64> {
    all_time_bucket(history, scope)?
        .account_value_history
        .iter()
        .rev()
        .find_map(|(_, value)| value.is_finite().then_some(*value))
}

fn selected_pnl_points(
    history: &PortfolioHistory,
    scope: PortfolioScope,
    window: PortfolioWindow,
    now_ms: u64,
) -> Vec<(u64, f64)> {
    let direct_key = direct_bucket_key(scope, window);
    let (bucket, used_direct_bucket) = direct_key
        .and_then(|key| history.buckets.get(key).map(|bucket| (bucket, true)))
        .or_else(|| all_time_bucket(history, scope).map(|bucket| (bucket, false)))
        .unwrap_or((&EMPTY_BUCKET, false));
    let points = sorted_finite_points(&bucket.pnl_history);

    if window == PortfolioWindow::AllTime || used_direct_bucket {
        points
    } else if let Some(cutoff) = window.cutoff_ms(now_ms) {
        apply_cutoff_with_baseline(&points, cutoff)
    } else {
        points
    }
}

static EMPTY_BUCKET: PortfolioBucket = PortfolioBucket {
    account_value_history: Vec::new(),
    pnl_history: Vec::new(),
    vlm: None,
    skipped_invalid_points: 0,
    invalid_vlm: false,
};

fn all_time_bucket(history: &PortfolioHistory, scope: PortfolioScope) -> Option<&PortfolioBucket> {
    let key = match scope {
        PortfolioScope::All => "allTime",
        PortfolioScope::Perp => "perpAllTime",
    };
    history.buckets.get(key)
}

fn direct_bucket_key(scope: PortfolioScope, window: PortfolioWindow) -> Option<&'static str> {
    match (scope, window) {
        (PortfolioScope::All, PortfolioWindow::Day) => Some("day"),
        (PortfolioScope::All, PortfolioWindow::Week) => Some("week"),
        (PortfolioScope::All, PortfolioWindow::Month) => Some("month"),
        (PortfolioScope::Perp, PortfolioWindow::Day) => Some("perpDay"),
        (PortfolioScope::Perp, PortfolioWindow::Week) => Some("perpWeek"),
        (PortfolioScope::Perp, PortfolioWindow::Month) => Some("perpMonth"),
        _ => None,
    }
}

fn sorted_finite_points(points: &[(u64, f64)]) -> Vec<(u64, f64)> {
    let mut sorted = points
        .iter()
        .copied()
        .filter(|(timestamp, value)| *timestamp > 0 && value.is_finite())
        .collect::<Vec<_>>();
    sorted.sort_by_key(|(timestamp, _)| *timestamp);
    sorted
        .into_iter()
        .fold(Vec::new(), |mut deduplicated, point| {
            if let Some(last) = deduplicated.last_mut()
                && last.0 == point.0
            {
                *last = point;
                return deduplicated;
            }
            deduplicated.push(point);
            deduplicated
        })
}

fn apply_cutoff_with_baseline(points: &[(u64, f64)], cutoff: u64) -> Vec<(u64, f64)> {
    let baseline = points
        .iter()
        .take_while(|(timestamp, _)| *timestamp <= cutoff)
        .last()
        .map(|(_, value)| *value);
    let mut filtered = points
        .iter()
        .copied()
        .filter(|(timestamp, _)| *timestamp >= cutoff)
        .collect::<Vec<_>>();

    if let Some(baseline) = baseline {
        if filtered.is_empty() {
            return vec![(cutoff, baseline)];
        }
        if filtered
            .first()
            .is_some_and(|(timestamp, _)| *timestamp > cutoff)
        {
            filtered.insert(0, (cutoff, baseline));
        }
    }
    filtered
}

fn portfolio_total_pnl(points: &[(u64, f64)]) -> Option<f64> {
    match points {
        [] => None,
        [(_, value)] => value.is_finite().then_some(*value),
        points => {
            let total = points.last()?.1 - points.first()?.1;
            total.is_finite().then_some(total)
        }
    }
}

fn aggregate_relative_series(series: &[Vec<(u64, f64)>]) -> Vec<(u64, f64)> {
    let relative = series
        .iter()
        .filter_map(|points| match points.as_slice() {
            [] => None,
            [only] => Some(vec![*only]),
            points => {
                let baseline = points.first()?.1;
                Some(
                    points
                        .iter()
                        .map(|(timestamp, value)| (*timestamp, *value - baseline))
                        .collect(),
                )
            }
        })
        .collect::<Vec<_>>();
    let timestamps = relative
        .iter()
        .flat_map(|points| points.iter().map(|(timestamp, _)| *timestamp))
        .collect::<BTreeSet<_>>();

    timestamps
        .into_iter()
        .filter_map(|timestamp| {
            let total = relative
                .iter()
                .filter_map(|points| {
                    points
                        .iter()
                        .take_while(|(point_timestamp, _)| *point_timestamp <= timestamp)
                        .last()
                        .map(|(_, value)| *value)
                })
                .sum::<f64>();
            total.is_finite().then_some((timestamp, total))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const ADDRESS_A: &str = "0x1111111111111111111111111111111111111111";

    fn history(points: &[(u64, f64)], account_value: f64) -> PortfolioHistory {
        PortfolioHistory {
            buckets: HashMap::from([(
                "allTime".to_string(),
                PortfolioBucket {
                    pnl_history: points.to_vec(),
                    account_value_history: vec![(1, account_value)],
                    ..PortfolioBucket::default()
                },
            )]),
        }
    }

    #[test]
    fn aggregation_aligns_different_wallet_timestamps_and_sums_totals() {
        let first = history(&[(10, 100.0), (20, 125.0), (40, 160.0)], 1_000.0);
        let second = history(&[(15, -5.0), (30, 15.0), (40, 25.0)], 2_000.0);

        let aggregate = aggregate_portfolios(
            &[&first, &second],
            PortfolioScope::All,
            PortfolioWindow::AllTime,
            50,
        );

        assert_eq!(aggregate.total_pnl, Some(90.0));
        assert_eq!(aggregate.account_value, Some(3_000.0));
        assert_eq!(aggregate.profitable_wallets, 2);
        assert_eq!(aggregate.points.last(), Some(&(40, 90.0)));
    }

    #[test]
    fn config_normalizes_and_deduplicates_wallet_addresses() {
        let config = CombinedPortfolioConfig {
            wallets: vec![
                TrackedWalletConfig {
                    address: ADDRESS_A.to_uppercase().replace("0X", "0x"),
                    label: " Primary ".to_string(),
                },
                TrackedWalletConfig {
                    address: ADDRESS_A.to_string(),
                    label: "Duplicate".to_string(),
                },
                TrackedWalletConfig {
                    address: "invalid".to_string(),
                    label: String::new(),
                },
            ],
            ..CombinedPortfolioConfig::default()
        };

        let state = CombinedPortfolioState::from_config(&config);

        assert_eq!(state.wallets.len(), 1);
        assert_eq!(state.wallets[0].address, ADDRESS_A);
        assert_eq!(state.wallets[0].label, "Primary");
    }

    #[test]
    fn rolling_window_uses_pre_cutoff_baseline() {
        const DAY_MS: u64 = 24 * 60 * 60 * 1000;
        let now = 20 * DAY_MS;
        let history = history(
            &[
                (10 * DAY_MS, 20.0),
                (14 * DAY_MS, 50.0),
                (18 * DAY_MS, 80.0),
                (20 * DAY_MS, 100.0),
            ],
            1_000.0,
        );

        assert_eq!(
            wallet_period_pnl(&history, PortfolioScope::All, PortfolioWindow::Week, now),
            Some(80.0)
        );
    }
}
