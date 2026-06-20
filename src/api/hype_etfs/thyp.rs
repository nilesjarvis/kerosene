use super::http::fetch_json;
use super::numbers::{finite, finite_positive, positive_ratio};
use crate::hype_etf_state::{HypeEtfDailyFlow, HypeEtfFund, HypeEtfTicker};

use serde::Deserialize;

const THYP_URL: &str =
    "https://21sharesprimary.paradox-coworking.com/api/product_valuation_history/thyp";

// ---------------------------------------------------------------------------
// THYP Feed
// ---------------------------------------------------------------------------

pub(super) async fn fetch_thyp() -> Result<HypeEtfFund, String> {
    let response: ThypResponse = fetch_json(THYP_URL, "THYP").await?;
    if !response.success {
        return Err("THYP response was not successful".to_string());
    }

    let latest = response
        .data
        .first()
        .ok_or_else(|| "THYP response did not include valuation history".to_string())?;

    Ok(thyp_fund_from_response(
        latest,
        &response.data,
        response.last_updated,
    ))
}

pub(super) fn thyp_fund_from_response(
    latest: &ThypValuation,
    history: &[ThypValuation],
    updated_at: Option<String>,
) -> HypeEtfFund {
    let hype_exposure = latest
        .total_nav
        .zip(latest.index)
        .and_then(|(total_nav, index)| positive_ratio(total_nav, index));

    HypeEtfFund {
        ticker: HypeEtfTicker::Thyp,
        as_of_date: Some(latest.valuation_date.clone()),
        updated_at,
        net_assets_usd: finite_positive(latest.total_nav),
        hype_exposure,
        shares_outstanding: finite_positive(latest.total_units_outstanding),
        nav_per_share: finite_positive(latest.nav_per_share),
        market_price: finite_positive(latest.market_price),
        nav_change_pct: finite(latest.nav_change_percentage),
        market_price_change_pct: finite(latest.market_price_percentage_change),
        premium_discount_pct: finite(latest.premium_discount),
        median_spread_pct: finite(latest.median_30d_spread).map(|value| value * 100.0),
        daily_volume: finite_positive(latest.daily_trading_volume),
        thirty_day_volume: finite_positive(latest.trading_volume_30d),
        hype_reference_price: finite_positive(latest.index),
        staking_net_rate_pct: None,
        staking_current_pct: None,
        daily_flows: thyp_daily_flows(history),
    }
}

pub(super) fn thyp_daily_flows(history: &[ThypValuation]) -> Vec<HypeEtfDailyFlow> {
    let mut history = history.iter().collect::<Vec<_>>();
    history.sort_by(|a, b| a.valuation_date.cmp(&b.valuation_date));

    let mut previous_units: Option<f64> = None;
    let mut flows = Vec::new();
    for valuation in history {
        let current_units = finite_positive(valuation.total_units_outstanding);
        if let (Some(previous_units), Some(current_units), Some(nav_per_share)) = (
            previous_units,
            current_units,
            finite_positive(valuation.nav_per_share),
        ) {
            let amount_usd = (current_units - previous_units) * nav_per_share;
            if amount_usd.is_finite() {
                flows.push(HypeEtfDailyFlow {
                    date: valuation.valuation_date.clone(),
                    amount_usd,
                });
            }
        }

        if let Some(current_units) = current_units {
            previous_units = Some(current_units);
        }
    }

    flows
}

#[derive(Deserialize)]
pub(super) struct ThypResponse {
    pub(super) success: bool,
    #[serde(default)]
    pub(super) data: Vec<ThypValuation>,
    #[serde(default, rename = "lastUpdated")]
    pub(super) last_updated: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct ThypValuation {
    pub(super) valuation_date: String,
    nav_per_share: Option<f64>,
    total_units_outstanding: Option<f64>,
    total_nav: Option<f64>,
    nav_change_percentage: Option<f64>,
    market_price: Option<f64>,
    market_price_percentage_change: Option<f64>,
    premium_discount: Option<f64>,
    index: Option<f64>,
    median_30d_spread: Option<f64>,
    daily_trading_volume: Option<f64>,
    trading_volume_30d: Option<f64>,
}
