use super::*;

fn balance(coin: &str, total: &str, entry_ntl: &str) -> SpotBalance {
    SpotBalance {
        coin: coin.to_string(),
        token: None,
        total: total.to_string(),
        hold: "0".to_string(),
        entry_ntl: entry_ntl.to_string(),
        supplied: None,
    }
}

#[test]
fn spot_equity_estimate_rejects_invalid_balance_numbers() {
    let mids = HashMap::new();
    assert_eq!(
        estimate_spot_equity(&[balance("USDC", "10", "0")], &mids),
        Some(10.0)
    );
    assert_eq!(
        estimate_spot_equity(&[balance("USDC", "bad", "0")], &mids),
        None
    );
    assert_eq!(
        estimate_spot_equity(&[balance("USDC", "NaN", "0")], &mids),
        None
    );
}

#[test]
fn spot_equity_estimate_requires_an_exact_spot_mark_for_non_stables() {
    let mut mids = HashMap::new();
    mids.insert(spot_balance_mid_key("PURR"), 4.0);
    assert_eq!(
        estimate_spot_equity(&[balance("PURR", "2", "3")], &mids),
        Some(8.0)
    );

    let mids = HashMap::new();
    assert_eq!(
        estimate_spot_equity(&[balance("PURR", "2", "3")], &mids),
        None
    );
    assert_eq!(
        estimate_spot_equity(&[balance("PURR", "2", "bad")], &mids),
        None
    );

    let mut mids = HashMap::new();
    mids.insert("PURR".to_string(), 0.0);
    assert_eq!(
        estimate_spot_equity(&[balance("PURR", "2", "3")], &mids),
        None
    );
}

#[test]
fn bare_perp_mid_is_never_used_for_spot_balance_equity() {
    let mids = HashMap::from([("HYPE".to_string(), 40.0)]);

    assert_eq!(
        estimate_spot_equity(&[balance("HYPE", "2", "50")], &mids),
        None
    );
}

#[test]
fn negative_spot_balances_reduce_equity_instead_of_disappearing() {
    let mids = HashMap::from([(spot_balance_mid_key("UBTC"), 60_000.0)]);

    assert_eq!(
        estimate_spot_equity(&[balance("UBTC", "-0.5", "0")], &mids),
        Some(-30_000.0)
    );
}

#[test]
fn validated_metadata_maps_indexed_spot_mid_to_balance_token() {
    let symbol = ExchangeSymbol {
        key: "@142".to_string(),
        ticker: "UBTC".to_string(),
        category: "spot".to_string(),
        display_name: Some("UBTC/USDC".to_string()),
        keywords: vec!["spot".to_string()],
        asset_index: 10_142,
        collateral_token: Some(crate::api::USDC_TOKEN_INDEX),
        sz_decimals: 5,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
    };
    let mut mids = HashMap::from([("@142".to_string(), 60_000.0)]);

    augment_spot_balance_mids(&mut mids, &[symbol]);

    assert_eq!(
        estimate_spot_equity(&[balance("UBTC", "0.5", "0")], &mids),
        Some(30_000.0)
    );
}

#[test]
fn api_named_spot_pair_mid_is_used_for_balance_valuation() {
    // allMids keys the canonical pair as "PURR/USDC", not "PURR".
    let mut mids = HashMap::new();
    mids.insert("PURR/USDC".to_string(), 4.0);

    assert_eq!(
        estimate_spot_equity(&[balance("PURR", "2", "3")], &mids),
        Some(8.0)
    );
}

#[test]
fn outcome_balance_uses_trade_coin_mid() {
    let mut mids = HashMap::new();
    mids.insert("#650".to_string(), 0.42);

    assert_eq!(
        estimate_spot_equity(&[balance("+650", "10", "1")], &mids),
        Some(4.2)
    );
}
