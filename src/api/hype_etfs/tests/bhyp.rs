use super::*;

#[test]
fn bhyp_mapping_normalizes_decimal_percentages() {
    let response = bhyp_response_or_panic(
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
    );

    let fund = bhyp_fund_from_response(response);

    assert_eq!(fund.ticker, HypeEtfTicker::Bhyp);
    assert_eq!(fund.hype_exposure, Some(95319.78615835));
    assert!((f64_or_panic(fund.premium_discount_pct, "premium discount") - 1.89).abs() < 0.0001);
    assert_eq!(fund.staking_net_rate_pct, Some(1.18125));
    assert_eq!(fund.staking_current_pct, Some(53.53022098741368));
    assert!(fund.daily_flows.is_empty());
}
