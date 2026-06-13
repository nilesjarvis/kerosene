use super::{ClosePositionInputError, close_position_order_side_and_size};
use crate::account::{
    AccountData, AccountDataCompleteness, AccountDataSection, AssetPosition, ClearinghouseState,
    MarginSummary, Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config::AccountProfile;
use crate::order_execution::{OneShotPlacementContext, OrderSurface};
use crate::order_update::PendingOneShotStatusRequest;
use crate::signing::ExchangeOrderKind;

mod inputs;
mod stale_account;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

fn connect_test_account(terminal: &mut TradingTerminal) {
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.wallet_address_input = TEST_ACCOUNT.to_string();
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: TEST_ACCOUNT.to_string(),
        agent_key: sensitive_string("").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }];
    terminal.active_account_index = 0;
}

fn account_data_with_position(coin: &str, fetched_at_ms: u64) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
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
        fetched_at_ms,
    }
}

fn terminal_with_stale_account() -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal
        .set_account_data_for_address_for_test(TEST_ACCOUNT, account_data_with_position("BTC", 1));
    terminal.account_loading = false;
    terminal
}

fn terminal_with_incomplete_fresh_account() -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("agent-key");
    let mut account_data = account_data_with_position("BTC", TradingTerminal::now_ms());
    account_data
        .completeness
        .mark_incomplete(AccountDataSection::Positions, "HIP-3 positions unavailable");
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, account_data);
    terminal.account_loading = false;
    terminal
}

fn order_status_or_panic(terminal: &TradingTerminal) -> (&str, bool) {
    match terminal.order_status.as_ref() {
        Some((message, is_error)) => (message.as_str(), *is_error),
        None => panic!("missing order status"),
    }
}

fn pending_one_shot_status_request() -> PendingOneShotStatusRequest {
    PendingOneShotStatusRequest::new(
        7,
        &OneShotPlacementContext {
            account_address: TEST_ACCOUNT.to_string(),
            cloid: "0x00000000000000000000000000000005".to_string(),
            surface: OrderSurface::Ticket,
            symbol_key: "BTC".to_string(),
            order_kind: ExchangeOrderKind::Market,
        },
    )
}

mod pending_indicator {
    use super::{TEST_ACCOUNT, account_data_with_position, connect_test_account};
    use crate::api::{ExchangeSymbol, MarketType};
    use crate::app_state::TradingTerminal;
    use crate::order_pending_indicators::PendingOrderIndicatorKind;

    fn btc_symbol() -> ExchangeSymbol {
        ExchangeSymbol {
            key: "BTC".to_string(),
            ticker: "BTC".to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 4,
            max_leverage: 50,
            only_isolated: false,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    fn terminal_with_fresh_position() -> TradingTerminal {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.pending_order_action = None;
        connect_test_account(&mut terminal);
        terminal.set_committed_agent_key_for_test("agent-key");
        terminal.account_loading = false;
        terminal.exchange_symbols = vec![btc_symbol()];
        terminal.set_account_data_for_address_for_test(
            TEST_ACCOUNT,
            account_data_with_position("BTC", TradingTerminal::now_ms()),
        );
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        terminal
    }

    #[test]
    fn market_close_creates_market_placing_indicator() {
        let mut terminal = terminal_with_fresh_position();

        let _task = terminal.execute_close_position("BTC", 1.0, true);

        assert_eq!(terminal.pending_order_indicators.len(), 1);
        let indicator = terminal
            .pending_order_indicators
            .values()
            .next()
            .expect("indicator should be created");
        assert_eq!(indicator.kind, PendingOrderIndicatorKind::MarketPlacing);
        assert_eq!(indicator.symbol, "BTC");
        assert_eq!(indicator.account_address, TEST_ACCOUNT);
        // Closing a long position sells.
        assert!(!indicator.is_buy);
    }

    #[test]
    fn limit_close_creates_placing_indicator() {
        let mut terminal = terminal_with_fresh_position();

        let _task = terminal.execute_close_position("BTC", 1.0, false);

        assert_eq!(terminal.pending_order_indicators.len(), 1);
        let indicator = terminal
            .pending_order_indicators
            .values()
            .next()
            .expect("indicator should be created");
        assert_eq!(indicator.kind, PendingOrderIndicatorKind::Placing);
        assert_eq!(indicator.account_address, TEST_ACCOUNT);
    }
}
