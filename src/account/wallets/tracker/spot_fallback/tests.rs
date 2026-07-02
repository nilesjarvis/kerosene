use super::*;
use crate::account::SpotBalance;

fn spot_balance(coin: &str, token: u32, total: &str) -> SpotBalance {
    SpotBalance {
        coin: coin.to_string(),
        token: Some(token),
        total: total.to_string(),
        hold: "0".to_string(),
        entry_ntl: "0".to_string(),
        supplied: None,
    }
}

fn pm_spot_state() -> SpotClearinghouseState {
    SpotClearinghouseState {
        balances: vec![
            spot_balance("USDC", 0, "250"),
            spot_balance("UBTC", 197, "0.5"),
        ],
        portfolio_margin_enabled: true,
        portfolio_margin_ratio: None,
        token_to_available_after_maintenance: Some(vec![
            (0, "180.5".to_string()),
            (197, "0.1".to_string()),
        ]),
    }
}

#[test]
fn fallback_needed_only_when_equity_or_withdrawable_not_positive() {
    assert!(!spot_equity_fallback_needed(Some(100.0), Some(50.0)));
    assert!(spot_equity_fallback_needed(Some(0.0), Some(50.0)));
    assert!(spot_equity_fallback_needed(Some(100.0), Some(0.0)));
    assert!(spot_equity_fallback_needed(Some(0.0), Some(0.0)));
    assert!(spot_equity_fallback_needed(None, None));
}

#[test]
fn pm_state_yields_spot_equity_and_token0_withdrawable() {
    // Unit-wrapped UBTC prices via the BTC mid; USDC counts at face value.
    let mids = HashMap::from([("BTC".to_string(), 60_000.0)]);
    let fallback = spot_equity_fallback_from_state(&pm_spot_state(), &mids)
        .expect("portfolio-margin state must produce a fallback");
    assert_eq!(fallback.equity, Some(250.0 + 0.5 * 60_000.0));
    assert_eq!(fallback.withdrawable, Some(180.5));
}

#[test]
fn non_pm_state_produces_no_fallback() {
    let mut spot = pm_spot_state();
    spot.portfolio_margin_enabled = false;
    assert!(spot_equity_fallback_from_state(&spot, &HashMap::new()).is_none());
}

#[test]
fn merge_fills_only_non_positive_perp_values() {
    let fallback = SpotEquityFallback {
        equity: Some(5_000.0),
        withdrawable: Some(1_000.0),
    };
    let mut equity = Some(0.0);
    let mut withdrawable = None;
    merge_spot_equity_fallback(Ok(Some(fallback)), &mut equity, &mut withdrawable);
    assert_eq!(equity, Some(5_000.0));
    assert_eq!(withdrawable, Some(1_000.0));

    // Already-positive perp values win over the spot estimate.
    let fallback = SpotEquityFallback {
        equity: Some(5_000.0),
        withdrawable: Some(1_000.0),
    };
    let mut equity = Some(42.0);
    let mut withdrawable = Some(7.0);
    merge_spot_equity_fallback(Ok(Some(fallback)), &mut equity, &mut withdrawable);
    assert_eq!(equity, Some(42.0));
    assert_eq!(withdrawable, Some(7.0));
}

#[test]
fn merge_keeps_zero_withdrawable_when_fallback_has_none() {
    let fallback = SpotEquityFallback {
        equity: Some(5_000.0),
        withdrawable: None,
    };
    let mut equity = Some(0.0);
    let mut withdrawable = Some(0.0);
    merge_spot_equity_fallback(Ok(Some(fallback)), &mut equity, &mut withdrawable);
    assert_eq!(equity, Some(5_000.0));
    assert_eq!(withdrawable, Some(0.0));
}

/// Regression: a failed auxiliary spotClearinghouseState/allMids request is
/// best-effort and must not discard the perp snapshot that was already
/// fetched successfully.
#[test]
fn merge_error_keeps_the_already_fetched_perp_values() {
    let mut equity = Some(1_234.5);
    let mut withdrawable = Some(0.0);
    merge_spot_equity_fallback(
        Err("spotClearinghouseState request failed: timeout".to_string()),
        &mut equity,
        &mut withdrawable,
    );
    assert_eq!(equity, Some(1_234.5));
    assert_eq!(withdrawable, Some(0.0));
}

#[test]
fn merge_non_pm_outcome_keeps_perp_values() {
    let mut equity = Some(0.0);
    let mut withdrawable = Some(0.0);
    merge_spot_equity_fallback(Ok(None), &mut equity, &mut withdrawable);
    assert_eq!(equity, Some(0.0));
    assert_eq!(withdrawable, Some(0.0));
}
