use crate::account::{AccountData, UserFeeRates};
use crate::app_state::TradingTerminal;
use crate::builder_codes::BuilderCodeConfig;

const ACCOUNT: &str = "0x1111111111111111111111111111111111111111";
const BUILDER: &str = "0x2222222222222222222222222222222222222222";

fn terminal_with_fees() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(ACCOUNT.into());
    terminal.account_data_address = Some(ACCOUNT.into());
    terminal.account_data = Some(AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: serde_json::from_value(serde_json::json!({
            "marginSummary": {"accountValue": "0", "totalNtlPos": "0", "totalMarginUsed": "0"},
            "withdrawable": "0", "assetPositions": []
        }))
        .expect("clearinghouse fixture"),
        clearinghouses_by_dex: Default::default(),
        spot: serde_json::from_value(serde_json::json!({"balances": []})).expect("spot fixture"),
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates {
            user_cross_rate: "0.0004".into(),
            user_add_rate: "0.0001".into(),
            user_spot_cross_rate: "0.0005".into(),
            user_spot_add_rate: "0.0002".into(),
            active_referral_discount: None,
        },
        completeness: Default::default(),
        fetched_at_ms: 1,
    });
    terminal.builder_codes.config = BuilderCodeConfig {
        enabled: true,
        address: BUILDER.into(),
        fee_tenths_bps: 15,
    };
    terminal
}

#[test]
fn builder_fee_estimates_include_perp_both_sides_and_spot_sells_only() {
    let terminal = terminal_with_fees();
    for (is_spot, is_buy, is_limit, expected_rate) in [
        (false, true, true, 0.00025),
        (false, false, true, 0.00025),
        (false, true, false, 0.00055),
        (false, false, false, 0.00055),
        (true, true, true, 0.0002),
        (true, false, true, 0.00035),
        (true, true, false, 0.0005),
        (true, false, false, 0.00065),
    ] {
        let (amount, rate) = terminal
            .estimate_fee(100.0, 2.0, is_limit, is_spot, is_buy)
            .expect("valid estimate");
        assert!((rate - expected_rate).abs() < 1e-12);
        assert!((amount - 200.0 * expected_rate).abs() < 1e-12);
    }
}

#[test]
fn builder_fee_estimates_do_not_silently_omit_invalid_enabled_configuration() {
    let mut terminal = terminal_with_fees();
    terminal.builder_codes.config.address = "invalid".into();
    assert!(
        terminal
            .estimate_fee(100.0, 2.0, false, false, true)
            .is_none()
    );

    terminal.builder_codes.config.enabled = false;
    assert_eq!(
        terminal.estimate_fee(100.0, 2.0, false, false, true),
        Some((0.08, 0.0004))
    );
}

#[test]
fn builder_fee_estimates_require_valid_quantity_price_and_account_identity() {
    let mut terminal = terminal_with_fees();
    for (price, quantity) in [(0.0, 1.0), (-100.0, -2.0), (f64::INFINITY, 1.0)] {
        assert!(
            terminal
                .estimate_fee(price, quantity, true, false, true)
                .is_none()
        );
    }
    terminal.account_data_address = Some(BUILDER.into());
    assert!(
        terminal
            .estimate_fee(100.0, 2.0, true, false, true)
            .is_none()
    );
}
