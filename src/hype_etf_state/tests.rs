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

#[test]
fn hype_etf_fund_debug_redacts_market_payload() {
    let mut fund = fund(HypeEtfTicker::Bhyp, 12_345_678.9, -0.1234);
    fund.as_of_date = Some("2026-06-18".to_string());
    fund.updated_at = Some("2026-06-18T12:34:56Z".to_string());
    fund.nav_per_share = Some(45.67);
    fund.market_price = Some(45.89);
    fund.nav_change_pct = Some(1.234);
    fund.market_price_change_pct = Some(2.345);
    fund.thirty_day_volume = Some(987_654.32);
    fund.hype_reference_price = Some(46.78);
    fund.staking_net_rate_pct = Some(3.456);
    fund.staking_current_pct = Some(4.567);
    fund.daily_flows = vec![HypeEtfDailyFlow {
        date: "2026-06-17".to_string(),
        amount_usd: 765_432.1,
    }];

    let rendered = format!("{fund:?}");

    assert!(rendered.contains("ticker: Bhyp"));
    assert!(rendered.contains("has_net_assets_usd: true"));
    assert!(rendered.contains("daily_flows_len: 1"));
    for secret in [
        "2026-06-18",
        "12,345,678.9",
        "12345678.9",
        "-0.1234",
        "45.67",
        "45.89",
        "987654.32",
        "765432.1",
    ] {
        assert!(!rendered.contains(secret), "ETF fund Debug leaked {secret}");
    }
}

#[test]
fn hype_etf_daily_flow_debug_redacts_amount() {
    let flow = HypeEtfDailyFlow {
        date: "2026-06-17".to_string(),
        amount_usd: 765_432.1,
    };

    let rendered = format!("{flow:?}");

    assert!(rendered.contains("date: \"2026-06-17\""));
    assert!(rendered.contains("amount_usd: \"<redacted>\""));
    assert!(!rendered.contains("765432.1"));
}

#[test]
fn hype_etf_data_debug_summarizes_funds_and_warnings() {
    let data = HypeEtfData {
        funds: vec![fund(HypeEtfTicker::Thyp, 12_345_678.9, -0.1234)],
        warnings: vec!["ETF warning secret".to_string()],
    };

    let rendered = format!("{data:?}");

    assert!(rendered.contains("funds_len: 1"));
    assert!(rendered.contains("fund_tickers: [Thyp]"));
    assert!(rendered.contains("warnings_len: 1"));
    assert!(!rendered.contains("12345678.9"));
    assert!(!rendered.contains("ETF warning secret"));
}

#[test]
fn hype_etf_state_debug_redacts_data_and_error() {
    let state = HypeEtfState {
        data: Some(HypeEtfData {
            funds: vec![fund(HypeEtfTicker::Bhyp, 12_345_678.9, -0.1234)],
            warnings: vec!["ETF warning secret".to_string()],
        }),
        error: Some("ETF error secret".to_string()),
        ..HypeEtfState::default()
    };

    let rendered = format!("{state:?}");

    assert!(rendered.contains("data_funds_len: Some(1)"));
    assert!(rendered.contains("error: Some(\"<redacted>\")"));
    assert!(!rendered.contains("12345678.9"));
    assert!(!rendered.contains("ETF warning secret"));
    assert!(!rendered.contains("ETF error secret"));
}

#[test]
fn hype_etf_totals_debug_redacts_financial_values() {
    let totals = HypeEtfTotals {
        net_assets_usd: Some(12_345_678.9),
        hype_exposure: Some(246_913.57),
        shares_outstanding: Some(10_000.0),
        daily_volume: Some(987_654.32),
        weighted_premium_discount_pct: Some(-0.1234),
        weighted_median_spread_pct: Some(0.5678),
    };

    let rendered = format!("{totals:?}");

    assert!(rendered.contains("has_net_assets_usd: true"));
    assert!(rendered.contains("has_weighted_median_spread_pct: true"));
    for secret in [
        "12345678.9",
        "246913.57",
        "10000",
        "987654.32",
        "-0.1234",
        "0.5678",
    ] {
        assert!(
            !rendered.contains(secret),
            "ETF totals Debug leaked {secret}"
        );
    }
}
