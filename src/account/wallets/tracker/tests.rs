use super::*;

use std::collections::HashMap;

fn portfolio(
    clearinghouse: serde_json::Value,
    spot: serde_json::Value,
) -> HydromancerPortfolioState {
    HydromancerPortfolioState::from_raw_for_tests(serde_json::json!({
        "clearinghouseState": clearinghouse,
        "spotClearinghouseState": spot,
        "userAbstraction": "default",
    }))
    .expect("portfolio parses")
}

fn clearinghouse_json(account_value: &str, withdrawable: &str) -> serde_json::Value {
    serde_json::json!({
        "marginSummary": {
            "accountValue": account_value,
            "totalNtlPos": "0",
            "totalMarginUsed": "0"
        },
        "withdrawable": withdrawable,
        "assetPositions": []
    })
}

/// Regression: a portfolio-margin wallet fetched through Hydromancer must
/// derive its equity/withdrawable from the spot clearinghouse state instead
/// of reporting the ~0 perp clearinghouse numbers.
#[test]
fn hydromancer_pm_wallet_derives_equity_from_spot_state() {
    let scope = AccountDataFetchScope::default();
    let portfolio = portfolio(
        clearinghouse_json("0.0", "0.0"),
        serde_json::json!({
            "balances": [
                { "coin": "USDC", "token": 0, "total": "1500", "hold": "0", "entryNtl": "0" },
                { "coin": "HYPE", "token": 150, "total": "100", "hold": "0", "entryNtl": "0" }
            ],
            "portfolioMarginEnabled": true,
            "tokenToAvailableAfterMaintenance": [[0, "1200.5"]]
        }),
    );

    let mut values = wallet_tracker_values_from_portfolio(portfolio, &scope).expect("values");
    let spot = values
        .spot_fallback
        .take()
        .expect("PM wallet must retain spot state for the fallback");

    let mids = HashMap::from([("HYPE".to_string(), 40.0)]);
    merge_spot_equity_fallback(
        Ok(spot_equity_fallback_from_state(&spot, &mids)),
        &mut values.equity,
        &mut values.withdrawable,
    );

    let snapshot = values.into_snapshot();
    assert_eq!(snapshot.equity, Some(1500.0 + 100.0 * 40.0));
    assert_eq!(snapshot.withdrawable, Some(1200.5));
}

#[test]
fn hydromancer_perp_wallet_snapshot_is_unchanged_and_skips_spot_fallback() {
    let scope = AccountDataFetchScope::default();
    let portfolio = portfolio(
        clearinghouse_json("2500.5", "1800.25"),
        serde_json::json!({ "balances": [], "portfolioMarginEnabled": false }),
    );

    let values = wallet_tracker_values_from_portfolio(portfolio, &scope).expect("values");
    assert!(values.spot_fallback.is_none());
    let snapshot = values.into_snapshot();
    assert_eq!(snapshot.equity, Some(2500.5));
    assert_eq!(snapshot.withdrawable, Some(1800.25));
}

#[test]
fn hydromancer_zero_equity_wallet_without_pm_skips_spot_fallback() {
    // Empty or fully-margined non-PM wallets keep their perp numbers; there
    // is no spot state to fall back to and no extra request to make.
    let portfolio = portfolio(
        clearinghouse_json("0.0", "0.0"),
        serde_json::json!({ "balances": [], "portfolioMarginEnabled": false }),
    );

    let values = wallet_tracker_values_from_portfolio(portfolio, &AccountDataFetchScope::default())
        .expect("values");
    assert!(values.spot_fallback.is_none());
    let snapshot = values.into_snapshot();
    assert_eq!(snapshot.equity, Some(0.0));
    assert_eq!(snapshot.withdrawable, Some(0.0));
}

#[test]
fn hydromancer_pm_wallet_with_positive_perp_numbers_skips_spot_fallback() {
    // Both equity and withdrawable already reflect real value: no fallback.
    let portfolio = portfolio(
        clearinghouse_json("900.0", "450.0"),
        serde_json::json!({
            "balances": [
                { "coin": "USDC", "token": 0, "total": "10", "hold": "0", "entryNtl": "0" }
            ],
            "portfolioMarginEnabled": true
        }),
    );

    let values = wallet_tracker_values_from_portfolio(portfolio, &AccountDataFetchScope::default())
        .expect("values");
    assert!(values.spot_fallback.is_none());
}
