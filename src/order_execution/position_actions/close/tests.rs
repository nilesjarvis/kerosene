use super::{ClosePositionInputError, close_position_order_side_and_size};
use crate::account::{
    AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
    Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
};
use crate::app_state::{TradingTerminal, sensitive_string};

fn account_data_with_position(coin: &str, fetched_at_ms: u64) -> AccountData {
    AccountData {
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "0".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "0".to_string(),
            asset_positions: vec![AssetPosition {
                position: Position {
                    coin: coin.to_string(),
                    szi: "1".to_string(),
                    entry_px: "100".to_string(),
                    position_value: "100".to_string(),
                    unrealized_pnl: "0".to_string(),
                    liquidation_px: None,
                    leverage: PositionLeverage {
                        leverage_type: "cross".to_string(),
                        value: 1,
                    },
                    margin_used: "0".to_string(),
                    cum_funding: None,
                },
                liquidation_px: None,
            }],
        },
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
        fetched_at_ms,
    }
}

fn terminal_with_stale_account() -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.wallet_key_input = sensitive_string("agent-key");
    terminal.account_data = Some(account_data_with_position("BTC", 1));
    terminal.account_loading = false;
    terminal
}

#[test]
fn close_position_refuses_stale_account_snapshot_and_requests_refresh() {
    let mut terminal = terminal_with_stale_account();

    let _task = terminal.execute_close_position("BTC", 1.0, true);

    let (message, is_error) = terminal.order_status.as_ref().expect("status");
    assert!(*is_error);
    assert!(message.contains("Account data is stale"));
    assert!(message.contains("refresh before closing positions"));
    assert!(terminal.account_loading);
}

#[test]
fn close_position_inputs_build_reduce_only_side_and_fractional_size() {
    assert_eq!(
        close_position_order_side_and_size("2.5", 0.5),
        Ok((false, "1.25".to_string()))
    );
    assert_eq!(
        close_position_order_side_and_size("-2.5", 1.0),
        Ok((true, "2.5".to_string()))
    );
}

#[test]
fn close_position_inputs_reject_invalid_position_sizes() {
    assert_eq!(
        close_position_order_side_and_size("abc", 0.5),
        Err(ClosePositionInputError::InvalidPositionSize)
    );
    assert_eq!(
        close_position_order_side_and_size("0", 0.5),
        Err(ClosePositionInputError::InvalidPositionSize)
    );
    assert_eq!(
        close_position_order_side_and_size("NaN", 0.5),
        Err(ClosePositionInputError::InvalidPositionSize)
    );
}

#[test]
fn close_position_inputs_reject_invalid_fractions() {
    assert_eq!(
        close_position_order_side_and_size("1", 0.0),
        Err(ClosePositionInputError::InvalidFraction)
    );
    assert_eq!(
        close_position_order_side_and_size("1", 1.25),
        Err(ClosePositionInputError::InvalidFraction)
    );
    assert_eq!(
        close_position_order_side_and_size("1", f64::NAN),
        Err(ClosePositionInputError::InvalidFraction)
    );
}
