use super::{
    moved_order_is_buy, moved_order_price_wire, moved_order_reduce_only, moved_order_size_wire,
};
use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, OpenOrder,
    SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::order_execution::{MoveOrderContextError, PendingMoveOrderContext};

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.rsplit(':').next().unwrap_or(key).to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 50,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn open_order(coin: &str, oid: u64, limit_px: &str) -> OpenOrder {
    OpenOrder {
        coin: coin.to_string(),
        side: "B".to_string(),
        limit_px: limit_px.to_string(),
        sz: "0.25".to_string(),
        oid,
        timestamp: 1,
        reduce_only: Some(false),
    }
}

fn account_data_with_order(order: OpenOrder) -> AccountData {
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
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: std::collections::HashMap::new(),
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: vec![order],
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms: 1,
    }
}

fn terminal_with_move_order(order_coin: &str, mid_coin: &str, mid: f64) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.wallet_key_input = sensitive_string("agent-key");
    terminal.muted_tickers.clear();
    terminal.exchange_symbols = vec![
        symbol(order_coin, MarketType::Perp),
        symbol("ETH", MarketType::Perp),
    ];
    terminal.account_data = Some(account_data_with_order(open_order(order_coin, 42, "100")));
    terminal.all_mids.clear();
    terminal.all_mids_updated_at_ms.clear();
    terminal.all_mids.insert(mid_coin.to_string(), mid);
    terminal
        .all_mids_updated_at_ms
        .insert(mid_coin.to_string(), TradingTerminal::now_ms());
    terminal
}

#[test]
fn moved_order_price_returns_none_when_rounded_price_is_unchanged() {
    assert_eq!(moved_order_price_wire(100.001, 100.0, 2, false), None);
}

#[test]
fn moved_order_price_returns_rounded_value_and_wire_price_when_rounded_price_changes() {
    assert_eq!(
        moved_order_price_wire(101.0, 100.0, 2, false),
        Some((101.0, "101".to_string()))
    );
}

#[test]
fn moved_order_price_rejects_nonfinite_new_price() {
    assert_eq!(moved_order_price_wire(f64::NAN, 100.0, 2, false), None);
    assert_eq!(moved_order_price_wire(f64::INFINITY, 100.0, 2, false), None);
}

#[test]
fn moved_order_price_rejects_invalid_original_price() {
    assert_eq!(moved_order_price_wire(101.0, f64::NAN, 2, false), None);
    assert_eq!(moved_order_price_wire(101.0, 0.0, 2, false), None);
    assert_eq!(moved_order_price_wire(101.0, -1.0, 2, false), None);
}

#[test]
fn moved_order_price_rejects_non_positive_rounded_new_price() {
    assert_eq!(moved_order_price_wire(0.0, 100.0, 2, false), None);
    assert_eq!(moved_order_price_wire(-1.0, 100.0, 2, false), None);
    assert_eq!(moved_order_price_wire(0.0000001, 100.0, 2, false), None);
}

#[test]
fn handle_move_order_blocks_far_away_drag_price() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);

    let _task = terminal.handle_move_order(42, 300.0);

    let (message, is_error) = terminal.order_status.as_ref().expect("status");
    assert!(*is_error);
    assert!(message.contains("away from BTC reference 100"));
    assert!(message.contains("Press Mid or update the price"));
    assert!(!terminal.pending_move_order_contexts.contains_key(&42));
}

#[test]
fn handle_move_order_fails_closed_when_dragged_order_mid_is_missing() {
    let mut terminal = terminal_with_move_order("BTC", "ETH", 100.0);
    terminal.active_symbol = "ETH".to_string();

    let _task = terminal.handle_move_order(42, 101.0);

    let (message, is_error) = terminal.order_status.as_ref().expect("status");
    assert!(*is_error);
    assert!(message.contains("No mid price for BTC"));
    assert!(!terminal.pending_move_order_contexts.contains_key(&42));
}

#[test]
fn handle_move_order_allows_in_band_drag_price() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);

    let _task = terminal.handle_move_order(42, 101.0);

    let (message, is_error) = terminal.order_status.as_ref().expect("status");
    assert!(!*is_error);
    assert_eq!(message, "Moving BTC order to $101...");
    assert!(terminal.pending_move_order_contexts.contains_key(&42));
}

#[test]
fn moved_order_size_returns_canonical_wire_size() {
    assert_eq!(moved_order_size_wire(" 0.25 "), Some("0.25".to_string()));
}

#[test]
fn moved_order_size_rejects_invalid_values() {
    assert_eq!(moved_order_size_wire(""), None);
    assert_eq!(moved_order_size_wire("abc"), None);
    assert_eq!(moved_order_size_wire("0"), None);
    assert_eq!(moved_order_size_wire("-1"), None);
    assert_eq!(moved_order_size_wire("NaN"), None);
    assert_eq!(moved_order_size_wire("inf"), None);
}

#[test]
fn moved_order_side_accepts_only_exchange_bid_or_ask_markers() {
    assert_eq!(moved_order_is_buy("B"), Some(true));
    assert_eq!(moved_order_is_buy("A"), Some(false));
    assert_eq!(moved_order_is_buy("buy"), None);
    assert_eq!(moved_order_is_buy(""), None);
}

#[test]
fn moved_order_reduce_only_preserves_known_perp_metadata() {
    assert_eq!(
        moved_order_reduce_only(MarketType::Perp, Some(true)),
        Ok(true)
    );
    assert_eq!(
        moved_order_reduce_only(MarketType::Perp, Some(false)),
        Ok(false)
    );
}

#[test]
fn moved_order_reduce_only_rejects_unknown_perp_metadata() {
    assert!(
        moved_order_reduce_only(MarketType::Perp, None)
            .expect_err("unknown reduce-only should be rejected")
            .contains("reduce-only metadata is unavailable")
    );
}

#[test]
fn moved_order_reduce_only_ignores_missing_spot_metadata() {
    assert_eq!(moved_order_reduce_only(MarketType::Spot, None), Ok(false));
}

#[test]
fn moved_order_reduce_only_ignores_missing_outcome_metadata() {
    assert_eq!(
        moved_order_reduce_only(MarketType::Outcome, None),
        Ok(false)
    );
}

#[test]
fn pending_move_context_reuses_captured_agent_key_for_same_account() {
    let context = PendingMoveOrderContext::new(
        "0xabc0000000000000000000000000000000000000",
        "original-agent-key",
    )
    .expect("valid context");

    assert_eq!(
        context.replacement_agent_key(Some("0xabc0000000000000000000000000000000000000")),
        Ok("original-agent-key".to_string().into())
    );
}

#[test]
fn pending_move_context_rejects_replacement_after_account_change() {
    let context = PendingMoveOrderContext::new(
        "0xabc0000000000000000000000000000000000000",
        "original-agent-key",
    )
    .expect("valid context");

    assert_eq!(
        context.replacement_agent_key(Some("0xdef0000000000000000000000000000000000000")),
        Err(MoveOrderContextError::AccountChanged)
    );
    assert_eq!(
        context.replacement_agent_key(None),
        Err(MoveOrderContextError::AccountChanged)
    );
}

#[test]
fn pending_move_context_rejects_empty_agent_key() {
    assert!(matches!(
        PendingMoveOrderContext::new("0xabc0000000000000000000000000000000000000", "   "),
        Err(MoveOrderContextError::MissingAgentKey)
    ));
}
