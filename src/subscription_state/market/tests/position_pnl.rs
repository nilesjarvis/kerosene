use super::*;
use crate::account::{
    AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
    Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
};

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

fn position(coin: &str, szi: &str) -> AssetPosition {
    AssetPosition {
        position: Position {
            coin: coin.to_string(),
            szi: szi.to_string(),
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
    }
}

fn account_data(positions: Vec<AssetPosition>) -> AccountData {
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
            asset_positions: positions,
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
        fetched_at_ms: 1_000,
    }
}

fn connect_with_positions(terminal: &mut TradingTerminal, positions: Vec<AssetPosition>) {
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, account_data(positions));
}

#[test]
fn position_pnl_subscriptions_require_toggle_key_and_open_positions() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![exchange_symbol("BTC", MarketType::Perp)];
    connect_with_positions(&mut terminal, vec![position("BTC", "1")]);

    let mut subscriptions = Vec::new();
    terminal.push_position_pnl_market_subscriptions(&mut subscriptions);
    assert!(subscriptions.is_empty());

    terminal.hydromancer_realtime_position_pnl_enabled = true;
    terminal.push_position_pnl_market_subscriptions(&mut subscriptions);
    assert!(subscriptions.is_empty());

    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.push_position_pnl_market_subscriptions(&mut subscriptions);
    assert_eq!(subscriptions.len(), 1);
}

#[test]
fn position_pnl_book_lagged_event_maps_to_account_message() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.hydromancer_api_key = "hydro-key".to_string().into();
    terminal.hydromancer_key_generation = 7;
    let source_context = terminal.hydromancer_keyed_market_data_source_context();
    let sigfigs = terminal.canonical_l2_book_sigfigs("BTC");

    let message = position_pnl_book_stream_event_message((
        source_context,
        crate::ws::KeyedBookStreamEvent::Lagged {
            id: 0,
            coin: "BTC".to_string(),
            sigfigs,
            hydromancer_key_generation: source_context.hydromancer_key_generation,
            skipped: 9,
        },
    ));

    match message {
        Message::PositionPnlWsBookLagged {
            coin,
            sigfigs: mapped_sigfigs,
            source_context: mapped_context,
            skipped,
        } => {
            assert_eq!(coin, "BTC");
            assert_eq!(mapped_sigfigs, sigfigs);
            assert_eq!(mapped_context, source_context);
            assert_eq!(skipped, 9);
        }
        other => panic!("expected position PnL book lagged message, got {other:?}"),
    }
}
