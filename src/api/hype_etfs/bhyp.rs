use super::http::fetch_json;
use super::numbers::{finite, finite_positive, positive_ratio};
use crate::hype_etf_state::{HypeEtfFund, HypeEtfTicker};

use serde::Deserialize;

const BHYP_URL: &str = "https://hyperliquidnews.xyz/api/bhyp";

// ---------------------------------------------------------------------------
// BHYP Feed
// ---------------------------------------------------------------------------

pub(super) async fn fetch_bhyp() -> Result<HypeEtfFund, String> {
    let response: BhypResponse = fetch_json(BHYP_URL, "BHYP").await?;
    Ok(bhyp_fund_from_response(response))
}

pub(super) fn bhyp_fund_from_response(response: BhypResponse) -> HypeEtfFund {
    let primary_holding = response
        .holdings
        .as_ref()
        .and_then(|holdings| holdings.basket.first());
    let holding_hype = primary_holding.and_then(|holding| finite_positive(holding.shares));
    let holding_market_value =
        primary_holding.and_then(|holding| finite_positive(holding.market_value));
    let holding_reference_price = holding_market_value
        .zip(holding_hype)
        .and_then(|(market_value, shares)| positive_ratio(market_value, shares));

    let fund_details = response.fund_details.as_ref();
    let premium_discount = response.premium_discount.as_ref();
    let nav_and_market_price = response.nav_and_market_price.as_ref();
    let staking = fund_details.and_then(|details| details.staking_metrics.as_ref());

    HypeEtfFund {
        ticker: HypeEtfTicker::Bhyp,
        as_of_date: fund_details
            .and_then(|details| details.as_of_date.clone())
            .or_else(|| premium_discount.and_then(|premium| premium.as_of_date.clone())),
        updated_at: response.updated_at.clone(),
        net_assets_usd: fund_details
            .and_then(|details| finite_positive(details.net_assets))
            .or(holding_market_value),
        hype_exposure: holding_hype,
        shares_outstanding: fund_details
            .and_then(|details| finite_positive(details.shares_outstanding)),
        nav_per_share: nav_and_market_price
            .and_then(|nav_market| finite_positive(nav_market.nav))
            .or_else(|| premium_discount.and_then(|premium| finite_positive(premium.nav))),
        market_price: nav_and_market_price
            .and_then(|nav_market| finite_positive(nav_market.market_price))
            .or_else(|| premium_discount.and_then(|premium| finite_positive(premium.market_price))),
        nav_change_pct: nav_and_market_price
            .and_then(|nav_market| nav_market.nav_change.as_ref())
            .and_then(|change| finite(change.percentage_change))
            .map(|value| value * 100.0),
        market_price_change_pct: nav_and_market_price
            .and_then(|nav_market| nav_market.market_price_change.as_ref())
            .and_then(|change| finite(change.percentage_change))
            .map(|value| value * 100.0),
        premium_discount_pct: premium_discount
            .and_then(|premium| finite(premium.premium_discount))
            .map(|value| value * 100.0),
        median_spread_pct: nav_and_market_price
            .and_then(|nav_market| finite(nav_market.thirty_day_median_bid_ask_spread))
            .map(|value| value * 100.0),
        daily_volume: premium_discount.and_then(|premium| finite_positive(premium.volume)),
        thirty_day_volume: None,
        hype_reference_price: holding_reference_price,
        staking_net_rate_pct: staking
            .and_then(|metrics| finite(metrics.net_staking_reward_rate))
            .map(|value| value * 100.0),
        staking_current_pct: staking
            .and_then(|metrics| finite(metrics.current_percentage_of_assets_staked))
            .map(|value| value * 100.0),
        // The BHYP feed currently exposes NAV history, but not historical share counts.
        daily_flows: Vec::new(),
    }
}

#[derive(Deserialize)]
pub(super) struct BhypResponse {
    #[serde(default, rename = "fundDetails")]
    fund_details: Option<BhypFundDetails>,
    #[serde(default)]
    holdings: Option<BhypHoldings>,
    #[serde(default, rename = "premiumDiscount")]
    premium_discount: Option<BhypPremiumDiscount>,
    #[serde(default, rename = "navAndMarketPrice")]
    nav_and_market_price: Option<BhypNavAndMarketPrice>,
    #[serde(default, rename = "updatedAt")]
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct BhypFundDetails {
    #[serde(default, rename = "netAssets")]
    net_assets: Option<f64>,
    #[serde(default, rename = "sharesOutstanding")]
    shares_outstanding: Option<f64>,
    #[serde(default, rename = "asOfDate")]
    as_of_date: Option<String>,
    #[serde(default, rename = "stakingMetrics")]
    staking_metrics: Option<BhypStakingMetrics>,
}

#[derive(Deserialize)]
struct BhypStakingMetrics {
    #[serde(default, rename = "netStakingRewardRate")]
    net_staking_reward_rate: Option<f64>,
    #[serde(default, rename = "currentPercentageOfAssetsStaked")]
    current_percentage_of_assets_staked: Option<f64>,
}

#[derive(Deserialize)]
struct BhypHoldings {
    #[serde(default)]
    basket: Vec<BhypHolding>,
}

#[derive(Deserialize)]
struct BhypHolding {
    #[serde(default)]
    shares: Option<f64>,
    #[serde(default, rename = "marketValue")]
    market_value: Option<f64>,
}

#[derive(Deserialize)]
struct BhypPremiumDiscount {
    nav: Option<f64>,
    #[serde(default, rename = "marketPrice")]
    market_price: Option<f64>,
    #[serde(default, rename = "premiumDiscount")]
    premium_discount: Option<f64>,
    #[serde(default, rename = "asOfDate")]
    as_of_date: Option<String>,
    volume: Option<f64>,
}

#[derive(Deserialize)]
struct BhypNavAndMarketPrice {
    nav: Option<f64>,
    #[serde(default, rename = "marketPrice")]
    market_price: Option<f64>,
    #[serde(default, rename = "navChange")]
    nav_change: Option<BhypChange>,
    #[serde(default, rename = "marketPriceChange")]
    market_price_change: Option<BhypChange>,
    #[serde(default, rename = "thirtyDayMedianBidAskSpread")]
    thirty_day_median_bid_ask_spread: Option<f64>,
}

#[derive(Deserialize)]
struct BhypChange {
    #[serde(default, rename = "percentageChange")]
    percentage_change: Option<f64>,
}
