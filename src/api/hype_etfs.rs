use super::CLIENT;
use crate::hype_etf_state::{HypeEtfDailyFlow, HypeEtfData, HypeEtfFund, HypeEtfTicker};

use flate2::read::GzDecoder;
use serde::Deserialize;
use std::io::Read;

const THYP_URL: &str =
    "https://21sharesprimary.paradox-coworking.com/api/product_valuation_history/thyp";
const BHYP_URL: &str = "https://hyperliquidnews.xyz/api/bhyp";

// ---------------------------------------------------------------------------
// HYPE ETF API
// ---------------------------------------------------------------------------

pub(crate) async fn fetch_hype_etfs() -> Result<HypeEtfData, String> {
    let (thyp, bhyp) = futures::future::join(fetch_thyp(), fetch_bhyp()).await;
    let mut funds = Vec::new();
    let mut warnings = Vec::new();

    match thyp {
        Ok(fund) => funds.push(fund),
        Err(error) => warnings.push(error),
    }
    match bhyp {
        Ok(fund) => funds.push(fund),
        Err(error) => warnings.push(error),
    }

    if funds.is_empty() {
        return Err(warnings.join("; "));
    }

    Ok(HypeEtfData { funds, warnings })
}

async fn fetch_thyp() -> Result<HypeEtfFund, String> {
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

async fn fetch_bhyp() -> Result<HypeEtfFund, String> {
    let response: BhypResponse = fetch_json(BHYP_URL, "BHYP").await?;
    Ok(bhyp_fund_from_response(response))
}

async fn fetch_json<T>(url: &str, label: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = CLIENT
        .clone()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("{label} request failed: {e}"))?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("{label} response read failed: {e}"))?;
    let text = decode_response_text(&bytes, label)?;

    if !status.is_success() {
        return Err(format!(
            "{label} request failed (HTTP {}): {}",
            status,
            response_snippet(&text)
        ));
    }

    serde_json::from_str(&text).map_err(|e| {
        format!(
            "{label} response parse failed: {e}; {}",
            response_snippet(&text)
        )
    })
}

fn response_snippet(text: &str) -> String {
    text.chars().take(200).collect()
}

fn decode_response_text(bytes: &[u8], label: &str) -> Result<String, String> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(bytes);
        let mut text = String::new();
        decoder
            .read_to_string(&mut text)
            .map_err(|e| format!("{label} gzip response decode failed: {e}"))?;
        return Ok(text);
    }

    String::from_utf8(bytes.to_vec()).map_err(|e| format!("{label} response was not UTF-8: {e}"))
}

fn thyp_fund_from_response(
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

fn bhyp_fund_from_response(response: BhypResponse) -> HypeEtfFund {
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

fn thyp_daily_flows(history: &[ThypValuation]) -> Vec<HypeEtfDailyFlow> {
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

fn finite(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite())
}

fn finite_positive(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn positive_ratio(numerator: f64, denominator: f64) -> Option<f64> {
    (numerator.is_finite() && denominator.is_finite() && denominator > 0.0)
        .then_some(numerator / denominator)
}

#[derive(Debug, Deserialize)]
struct ThypResponse {
    success: bool,
    #[serde(default)]
    data: Vec<ThypValuation>,
    #[serde(default, rename = "lastUpdated")]
    last_updated: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ThypValuation {
    valuation_date: String,
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

#[derive(Debug, Deserialize)]
struct BhypResponse {
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
struct BhypStakingMetrics {
    #[serde(default, rename = "netStakingRewardRate")]
    net_staking_reward_rate: Option<f64>,
    #[serde(default, rename = "currentPercentageOfAssetsStaked")]
    current_percentage_of_assets_staked: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct BhypHoldings {
    #[serde(default)]
    basket: Vec<BhypHolding>,
}

#[derive(Debug, Deserialize)]
struct BhypHolding {
    #[serde(default)]
    shares: Option<f64>,
    #[serde(default, rename = "marketValue")]
    market_value: Option<f64>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
struct BhypChange {
    #[serde(default, rename = "percentageChange")]
    percentage_change: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thyp_mapping_uses_latest_history_and_estimates_hype_exposure() {
        let response: ThypResponse = serde_json::from_str(
            r#"{
                "success": true,
                "lastUpdated": "2026-05-19T20:03:57.280Z",
                "data": [{
                    "valuation_date": "2026-05-18",
                    "nav_per_share": 26.37,
                    "total_units_outstanding": 540000,
                    "total_nav": 14240451.27,
                    "nav_change_percentage": 1.972,
                    "market_price": 26.865,
                    "market_price_percentage_change": 3.902,
                    "premium_discount": 1.8243,
                    "index": 45.57,
                    "median_30d_spread": 0.0023121387,
                    "daily_trading_volume": 356983,
                    "trading_volume_30d": 221848.6
                }]
            }"#,
        )
        .expect("json");

        let fund =
            thyp_fund_from_response(&response.data[0], &response.data, response.last_updated);

        assert_eq!(fund.ticker, HypeEtfTicker::Thyp);
        assert_eq!(fund.net_assets_usd, Some(14240451.27));
        assert_eq!(fund.hype_reference_price, Some(45.57));
        assert_eq!(fund.median_spread_pct, Some(0.23121387));
        assert!(fund.hype_exposure.unwrap() > 312_000.0);
        assert!(fund.daily_flows.is_empty());
    }

    #[test]
    fn thyp_daily_flows_track_share_creations_in_date_order() {
        let response: ThypResponse = serde_json::from_str(
            r#"{
                "success": true,
                "data": [
                    {
                        "valuation_date": "2026-05-15",
                        "nav_per_share": 25.86,
                        "total_units_outstanding": 450000,
                        "total_nav": 11636590.16,
                        "nav_change_percentage": 0.271,
                        "market_price": 25.856,
                        "market_price_percentage_change": -0.324,
                        "premium_discount": 0,
                        "index": 44.69,
                        "median_30d_spread": 0.0023273856,
                        "daily_trading_volume": 224266,
                        "trading_volume_30d": 188065
                    },
                    {
                        "valuation_date": "2026-05-14",
                        "nav_per_share": 25.79,
                        "total_units_outstanding": 330000,
                        "total_nav": 8509656.59,
                        "nav_change_percentage": 14.014,
                        "market_price": 25.94,
                        "market_price_percentage_change": 14.551,
                        "premium_discount": 0.5783,
                        "index": 44.56,
                        "median_30d_spread": 0.0022177866,
                        "daily_trading_volume": 333704,
                        "trading_volume_30d": 175998
                    }
                ]
            }"#,
        )
        .expect("json");

        let flows = thyp_daily_flows(&response.data);

        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].date, "2026-05-15");
        assert!((flows[0].amount_usd - 3_103_200.0).abs() < 0.01);
    }

    #[test]
    fn bhyp_mapping_normalizes_decimal_percentages() {
        let response: BhypResponse = serde_json::from_str(
            r#"{
                "fundDetails": {
                    "netAssets": 4344622.59,
                    "sharesOutstanding": 170000,
                    "asOfDate": "2026-05-18",
                    "stakingMetrics": {
                        "netStakingRewardRate": 0.0118125,
                        "currentPercentageOfAssetsStaked": 0.5353022098741368
                    }
                },
                "holdings": {
                    "basket": [{
                        "shares": 95319.78615835,
                        "marketValue": 4344761.64
                    }]
                },
                "premiumDiscount": {
                    "nav": 25.56,
                    "marketPrice": 26.04,
                    "premiumDiscount": 0.0189,
                    "asOfDate": "2026-05-18",
                    "volume": 103109
                },
                "navAndMarketPrice": {
                    "navChange": { "percentageChange": 0.020244740933750043 },
                    "marketPriceChange": { "percentageChange": 0.039936102236421724 },
                    "thirtyDayMedianBidAskSpread": 0.0024213075000000003
                },
                "updatedAt": "2026-05-19T20:04:35.941Z"
            }"#,
        )
        .expect("json");

        let fund = bhyp_fund_from_response(response);

        assert_eq!(fund.ticker, HypeEtfTicker::Bhyp);
        assert_eq!(fund.hype_exposure, Some(95319.78615835));
        assert!((fund.premium_discount_pct.unwrap() - 1.89).abs() < 0.0001);
        assert_eq!(fund.staking_net_rate_pct, Some(1.18125));
        assert_eq!(fund.staking_current_pct, Some(53.53022098741368));
        assert!(fund.daily_flows.is_empty());
    }
}
