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
fn spot_equity_estimate_requires_mid_or_valid_entry_notional_for_non_stables() {
    let mut mids = HashMap::new();
    mids.insert("PURR".to_string(), 4.0);
    assert_eq!(
        estimate_spot_equity(&[balance("PURR", "2", "3")], &mids),
        Some(8.0)
    );

    let mids = HashMap::new();
    assert_eq!(
        estimate_spot_equity(&[balance("PURR", "2", "3")], &mids),
        Some(3.0)
    );
    assert_eq!(
        estimate_spot_equity(&[balance("PURR", "2", "bad")], &mids),
        None
    );

    let mut mids = HashMap::new();
    mids.insert("PURR".to_string(), 0.0);
    assert_eq!(
        estimate_spot_equity(&[balance("PURR", "2", "3")], &mids),
        Some(3.0)
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
