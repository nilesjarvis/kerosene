use crate::helpers::{finite_value, positive_finite_value};

use std::{collections::BTreeMap, time::Instant};

// ---------------------------------------------------------------------------
// HYPE ETF State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum HypeEtfView {
    #[default]
    All,
    Thyp,
    Bhyp,
}

impl HypeEtfView {
    pub(crate) const ALL: [Self; 3] = [Self::All, Self::Thyp, Self::Bhyp];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Thyp => "THYP",
            Self::Bhyp => "BHYP",
        }
    }

    pub(crate) fn includes(self, ticker: HypeEtfTicker) -> bool {
        match self {
            Self::All => true,
            Self::Thyp => ticker == HypeEtfTicker::Thyp,
            Self::Bhyp => ticker == HypeEtfTicker::Bhyp,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HypeEtfTicker {
    Thyp,
    Bhyp,
}

impl HypeEtfTicker {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Thyp => "THYP",
            Self::Bhyp => "BHYP",
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Thyp => "21Shares HYPE",
            Self::Bhyp => "Bitwise HYPE",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HypeEtfState {
    pub(crate) view: HypeEtfView,
    pub(crate) data: Option<HypeEtfData>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) last_fetch: Option<Instant>,
    pub(crate) refresh_request_id: u64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HypeEtfData {
    pub(crate) funds: Vec<HypeEtfFund>,
    pub(crate) warnings: Vec<String>,
}

impl HypeEtfData {
    pub(crate) fn selected_funds(&self, view: HypeEtfView) -> Vec<&HypeEtfFund> {
        self.funds
            .iter()
            .filter(|fund| view.includes(fund.ticker))
            .collect()
    }

    pub(crate) fn totals_for(&self, view: HypeEtfView) -> HypeEtfTotals {
        HypeEtfTotals::from_funds(self.selected_funds(view))
    }

    pub(crate) fn daily_flows_for(&self, view: HypeEtfView) -> Vec<HypeEtfDailyFlow> {
        let mut flows_by_date = BTreeMap::new();
        for fund in self.funds.iter().filter(|fund| view.includes(fund.ticker)) {
            for flow in &fund.daily_flows {
                let Some(amount_usd) = finite_value(flow.amount_usd) else {
                    continue;
                };
                *flows_by_date.entry(flow.date.clone()).or_insert(0.0) += amount_usd;
            }
        }

        flows_by_date
            .into_iter()
            .map(|(date, amount_usd)| HypeEtfDailyFlow { date, amount_usd })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HypeEtfFund {
    pub(crate) ticker: HypeEtfTicker,
    pub(crate) as_of_date: Option<String>,
    pub(crate) updated_at: Option<String>,
    pub(crate) net_assets_usd: Option<f64>,
    pub(crate) hype_exposure: Option<f64>,
    pub(crate) shares_outstanding: Option<f64>,
    pub(crate) nav_per_share: Option<f64>,
    pub(crate) market_price: Option<f64>,
    pub(crate) nav_change_pct: Option<f64>,
    pub(crate) market_price_change_pct: Option<f64>,
    pub(crate) premium_discount_pct: Option<f64>,
    pub(crate) median_spread_pct: Option<f64>,
    pub(crate) daily_volume: Option<f64>,
    pub(crate) thirty_day_volume: Option<f64>,
    pub(crate) hype_reference_price: Option<f64>,
    pub(crate) staking_net_rate_pct: Option<f64>,
    pub(crate) staking_current_pct: Option<f64>,
    pub(crate) daily_flows: Vec<HypeEtfDailyFlow>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HypeEtfDailyFlow {
    pub(crate) date: String,
    pub(crate) amount_usd: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct HypeEtfTotals {
    pub(crate) net_assets_usd: Option<f64>,
    pub(crate) hype_exposure: Option<f64>,
    pub(crate) shares_outstanding: Option<f64>,
    pub(crate) daily_volume: Option<f64>,
    pub(crate) weighted_premium_discount_pct: Option<f64>,
    pub(crate) weighted_median_spread_pct: Option<f64>,
}

impl HypeEtfTotals {
    fn from_funds(funds: Vec<&HypeEtfFund>) -> Self {
        let net_assets_usd = sum_options(funds.iter().filter_map(|fund| fund.net_assets_usd));
        let hype_exposure = sum_options(funds.iter().filter_map(|fund| fund.hype_exposure));
        let shares_outstanding =
            sum_options(funds.iter().filter_map(|fund| fund.shares_outstanding));
        let daily_volume = sum_options(funds.iter().filter_map(|fund| fund.daily_volume));
        let weighted_premium_discount_pct = weighted_average(&funds, |fund| {
            fund.premium_discount_pct.zip(fund.net_assets_usd)
        });
        let weighted_median_spread_pct = weighted_average(&funds, |fund| {
            fund.median_spread_pct.zip(fund.net_assets_usd)
        });

        Self {
            net_assets_usd,
            hype_exposure,
            shares_outstanding,
            daily_volume,
            weighted_premium_discount_pct,
            weighted_median_spread_pct,
        }
    }
}

fn sum_options(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut total = 0.0;
    let mut has_value = false;
    for value in values.filter_map(finite_value) {
        total += value;
        has_value = true;
    }
    has_value.then_some(total)
}

fn weighted_average(
    funds: &[&HypeEtfFund],
    value_and_weight: impl Fn(&HypeEtfFund) -> Option<(f64, f64)>,
) -> Option<f64> {
    let mut weighted_total = 0.0;
    let mut weight_total = 0.0;
    for fund in funds {
        let Some((value, weight)) = value_and_weight(fund) else {
            continue;
        };
        let Some(value) = finite_value(value) else {
            continue;
        };
        let Some(weight) = positive_finite_value(weight) else {
            continue;
        };
        weighted_total += value * weight;
        weight_total += weight;
    }

    (weight_total > 0.0).then_some(weighted_total / weight_total)
}

#[cfg(test)]
mod tests;
