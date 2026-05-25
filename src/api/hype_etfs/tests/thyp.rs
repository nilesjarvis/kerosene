use super::*;

#[test]
fn thyp_mapping_uses_latest_history_and_estimates_hype_exposure() {
    let response = thyp_response_or_panic(
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
    );

    let fund = thyp_fund_from_response(&response.data[0], &response.data, response.last_updated);

    assert_eq!(fund.ticker, HypeEtfTicker::Thyp);
    assert_eq!(fund.net_assets_usd, Some(14240451.27));
    assert_eq!(fund.hype_reference_price, Some(45.57));
    assert_eq!(fund.median_spread_pct, Some(0.23121387));
    assert!(f64_or_panic(fund.hype_exposure, "hype exposure") > 312_000.0);
    assert!(fund.daily_flows.is_empty());
}

#[test]
fn thyp_daily_flows_track_share_creations_in_date_order() {
    let response = thyp_response_or_panic(
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
    );

    let flows = thyp_daily_flows(&response.data);

    assert_eq!(flows.len(), 1);
    assert_eq!(flows[0].date, "2026-05-15");
    assert!((flows[0].amount_usd - 3_103_200.0).abs() < 0.01);
}
