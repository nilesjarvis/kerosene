use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, OpenOrder,
    SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
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

pub(super) fn outcome_symbol(key: &str, is_question_fallback: bool) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 100_000_000,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 66,
            question_id: Some(12),
            question_name: Some("Recurring".to_string()),
            question_description: None,
            question_class: Some("priceBucket".to_string()),
            question_underlying: Some("BTC".to_string()),
            question_expiry: Some("20260520-0600".to_string()),
            question_price_thresholds: vec!["75348".to_string(), "78423".to_string()],
            question_period: Some("1d".to_string()),
            question_named_outcomes: vec![67, 68, 69],
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: Some(66),
            bucket_index: None,
            is_question_fallback,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring Fallback".to_string(),
            description: "other".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 660,
        }),
    }
}

pub(super) fn open_order(coin: &str, oid: u64, limit_px: &str) -> OpenOrder {
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

pub(super) fn account_data_with_order(order: OpenOrder) -> AccountData {
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

pub(super) fn terminal_with_move_order(
    order_coin: &str,
    mid_coin: &str,
    mid: f64,
) -> TradingTerminal {
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

pub(super) fn order_status_or_panic(terminal: &TradingTerminal) -> (&str, bool) {
    match &terminal.order_status {
        Some((message, is_error)) => (message.as_str(), *is_error),
        None => panic!("status"),
    }
}

pub(super) fn context_or_panic(
    result: Result<PendingMoveOrderContext, MoveOrderContextError>,
) -> PendingMoveOrderContext {
    match result {
        Ok(context) => context,
        Err(error) => panic!("valid context: {error:?}"),
    }
}

pub(super) fn reduce_only_error_or_panic(result: Result<bool, &'static str>) -> &'static str {
    match result {
        Ok(value) => panic!("unknown reduce-only should be rejected, got {value}"),
        Err(error) => error,
    }
}
