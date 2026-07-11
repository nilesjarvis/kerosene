use super::*;

fn position() -> Position {
    Position {
        coin: "SECRETPOSITIONCOIN".to_string(),
        szi: "position-size-secret".to_string(),
        entry_px: "entry-price-secret".to_string(),
        position_value: "position-value-secret".to_string(),
        unrealized_pnl: "pnl-secret".to_string(),
        liquidation_px: Some("liquidation-price-secret".to_string()),
        leverage: PositionLeverage {
            leverage_type: "cross".to_string(),
            value: 7,
        },
        margin_used: "margin-secret".to_string(),
        cum_funding: Some(CumFunding {
            since_open: "funding-secret".to_string(),
        }),
    }
}

#[test]
fn position_debug_redacts_financial_payload() {
    let position = position();

    let rendered = format!("{position:?}");

    assert!(rendered.contains("Position"));
    assert!(rendered.contains("has_liquidation_px: true"));
    assert!(rendered.contains("has_cum_funding: true"));
    assert!(rendered.contains("leverage_type: \"cross\""));
    assert!(rendered.contains("value: \"<redacted>\""));
    assert!(!rendered.contains("value: 7"));
    for secret in [
        "SECRETPOSITIONCOIN",
        "position-size-secret",
        "entry-price-secret",
        "position-value-secret",
        "pnl-secret",
        "liquidation-price-secret",
        "margin-secret",
        "funding-secret",
    ] {
        assert!(!rendered.contains(secret), "position Debug leaked {secret}");
    }
}

#[test]
fn clearinghouse_state_debug_summarizes_account_payload() {
    let state = ClearinghouseState {
        margin_summary: MarginSummary {
            account_value: "account-value-secret".to_string(),
            total_ntl_pos: "notional-secret".to_string(),
            total_margin_used: "margin-used-secret".to_string(),
        },
        cross_margin_summary: Some(MarginSummary {
            account_value: "cross-account-value-secret".to_string(),
            total_ntl_pos: "cross-notional-secret".to_string(),
            total_margin_used: "cross-margin-used-secret".to_string(),
        }),
        cross_maintenance_margin_used: Some("maintenance-secret".to_string()),
        withdrawable: "withdrawable-secret".to_string(),
        asset_positions: vec![AssetPosition {
            position: position(),
            liquidation_px: Some("wrapper-liquidation-secret".to_string()),
        }],
    };

    let rendered = format!("{state:?}");

    assert!(rendered.contains("ClearinghouseState"));
    assert!(rendered.contains("has_cross_margin_summary: true"));
    assert!(rendered.contains("has_cross_maintenance_margin_used: true"));
    assert!(rendered.contains("asset_positions_count: 1"));
    for secret in [
        "account-value-secret",
        "notional-secret",
        "margin-used-secret",
        "cross-account-value-secret",
        "cross-notional-secret",
        "cross-margin-used-secret",
        "maintenance-secret",
        "withdrawable-secret",
        "SECRETPOSITIONCOIN",
        "wrapper-liquidation-secret",
    ] {
        assert!(
            !rendered.contains(secret),
            "clearinghouse Debug leaked {secret}"
        );
    }
}

#[test]
fn funding_entry_debug_redacts_financial_payload() {
    let entry = FundingEntry {
        delta: FundingDelta {
            coin: "SECRETFUNDINGCOIN".to_string(),
            funding_rate: "funding-rate-secret".to_string(),
            szi: "funding-size-secret".to_string(),
            usdc: "funding-usdc-secret".to_string(),
        },
        time: 123,
    };

    let rendered = format!("{entry:?}");

    assert!(rendered.contains("FundingEntry"));
    assert!(rendered.contains("time: 123"));
    for secret in [
        "SECRETFUNDINGCOIN",
        "funding-rate-secret",
        "funding-size-secret",
        "funding-usdc-secret",
    ] {
        assert!(!rendered.contains(secret), "funding Debug leaked {secret}");
    }
}
