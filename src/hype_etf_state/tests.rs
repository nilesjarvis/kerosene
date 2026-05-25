use super::*;

mod daily_flows;
mod selection;
mod totals;

fn fund(ticker: HypeEtfTicker, net_assets_usd: f64, premium_discount_pct: f64) -> HypeEtfFund {
    HypeEtfFund {
        ticker,
        as_of_date: None,
        updated_at: None,
        net_assets_usd: Some(net_assets_usd),
        hype_exposure: Some(net_assets_usd / 50.0),
        shares_outstanding: Some(10.0),
        nav_per_share: None,
        market_price: None,
        nav_change_pct: None,
        market_price_change_pct: None,
        premium_discount_pct: Some(premium_discount_pct),
        median_spread_pct: Some(0.2),
        daily_volume: Some(1_000.0),
        thirty_day_volume: None,
        hype_reference_price: None,
        staking_net_rate_pct: None,
        staking_current_pct: None,
        daily_flows: Vec::new(),
    }
}
