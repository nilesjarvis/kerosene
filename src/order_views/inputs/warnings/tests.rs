use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
    SpotClearinghouseState, UserFeeRates,
};
use crate::app_state::TradingTerminal;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

fn account_data_with_withdrawable(withdrawable: &str) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: withdrawable.to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: withdrawable.to_string(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: std::collections::HashMap::new(),
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms: TradingTerminal::now_ms(),
    }
}

fn connected_terminal(withdrawable: &str) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_withdrawable(withdrawable),
    );
    terminal
}

#[test]
fn leverage_warning_is_suppressed_for_spot_orders() {
    let terminal = connected_terminal("1000");

    assert!(
        terminal
            .leverage_warning(true, false, Some(10_000.0))
            .is_none()
    );
}

#[test]
fn leverage_warning_is_suppressed_for_outcome_orders() {
    let terminal = connected_terminal("1000");

    assert!(
        terminal
            .leverage_warning(false, true, Some(10_000.0))
            .is_none()
    );
}

#[test]
fn leverage_warning_flags_high_perp_notional_against_margin() {
    let terminal = connected_terminal("1000");

    let (message, _) = terminal
        .leverage_warning(false, false, Some(10_000.0))
        .expect("high perp notional should warn");

    assert_eq!(message, "High Leverage: 10.0x Notional");
}

#[test]
fn leverage_warning_stays_quiet_for_perp_notional_within_margin() {
    let terminal = connected_terminal("1000");

    assert!(
        terminal
            .leverage_warning(false, false, Some(1_500.0))
            .is_none()
    );
}
