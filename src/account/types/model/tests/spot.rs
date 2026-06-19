use super::*;

#[test]
fn spot_balance_preserves_optional_token_index() {
    let balance = spot_balance_or_panic(serde_json::json!({
        "coin": "USDC",
        "token": 0,
        "total": "10",
        "hold": "2",
        "entryNtl": "0"
    }));

    assert_eq!(balance.token, Some(0));
}

#[test]
fn spot_balance_debug_redacts_financial_payload() {
    let balance = SpotBalance {
        coin: "SECRETSPOTCOIN".to_string(),
        token: Some(7),
        total: "spot-total-secret".to_string(),
        hold: "spot-hold-secret".to_string(),
        entry_ntl: "spot-entry-secret".to_string(),
        supplied: Some("spot-supplied-secret".to_string()),
    };

    let rendered = format!("{balance:?}");

    assert!(rendered.contains("SpotBalance"));
    assert!(rendered.contains("token: Some(7)"));
    assert!(rendered.contains("has_supplied: true"));
    for secret in [
        "SECRETSPOTCOIN",
        "spot-total-secret",
        "spot-hold-secret",
        "spot-entry-secret",
        "spot-supplied-secret",
    ] {
        assert!(
            !rendered.contains(secret),
            "spot balance Debug leaked {secret}"
        );
    }
}

#[test]
fn spot_clearinghouse_debug_summarizes_balances() {
    let state = SpotClearinghouseState {
        balances: vec![SpotBalance {
            coin: "SECRETSPOTCOIN".to_string(),
            token: Some(7),
            total: "spot-total-secret".to_string(),
            hold: "spot-hold-secret".to_string(),
            entry_ntl: "spot-entry-secret".to_string(),
            supplied: None,
        }],
        portfolio_margin_enabled: true,
        portfolio_margin_ratio: Some("portfolio-ratio-secret".to_string()),
        token_to_available_after_maintenance: Some(vec![(7, "available-secret".to_string())]),
    };

    let rendered = format!("{state:?}");

    assert!(rendered.contains("SpotClearinghouseState"));
    assert!(rendered.contains("balances_count: 1"));
    assert!(rendered.contains("portfolio_margin_enabled: true"));
    assert!(rendered.contains("has_portfolio_margin_ratio: true"));
    assert!(rendered.contains("token_to_available_after_maintenance_count: Some(1)"));
    for secret in [
        "SECRETSPOTCOIN",
        "spot-total-secret",
        "portfolio-ratio-secret",
        "available-secret",
    ] {
        assert!(
            !rendered.contains(secret),
            "spot clearinghouse Debug leaked {secret}"
        );
    }
}
