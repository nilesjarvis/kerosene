use super::*;
use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
    SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, OutcomeSymbolInfo};
use crate::signing::ExchangeOrderKind;

mod cancel;
mod modify;
mod place;

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 7,
        collateral_token: None,
        sz_decimals: 4,
        max_leverage: 50,
        only_isolated: false,
        growth_mode: false,
        market_type,
        outcome: None,
    }
}

fn outcome_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        sz_decimals: 0,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 65,
            contract: crate::api::OutcomeContract::verified_fixture(),
            venue: None,
            question_id: Some(12),
            question_name: Some("Recurring".to_string()),
            question_description: None,
            question_class: Some("priceBucket".to_string()),
            question_underlying: Some("BTC".to_string()),
            question_expiry: Some("20260520-0600".to_string()),
            question_price_thresholds: Vec::new(),
            question_period: None,
            question_named_outcomes: Vec::new(),
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: None,
            bucket_index: Some(0),
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring Named Outcome".to_string(),
            description: "index:0".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDC".to_string(),
            quote_token_index: Some(crate::api::USDC_TOKEN_INDEX),
            encoding: 650,
        }),
        ..symbol(key, MarketType::Outcome)
    }
}

fn incomplete_perp_account_data() -> AccountData {
    let mut completeness = AccountDataCompleteness::default();
    completeness.positions_complete = false;
    completeness.positions_actionable = false;
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
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates::default(),
        completeness,
        fetched_at_ms: TradingTerminal::now_ms(),
    }
}

fn ticket_limit_intent(symbol_key: &str) -> PlaceIntent {
    PlaceIntent {
        surface: OrderSurface::Ticket,
        symbol_key: symbol_key.to_string(),
        is_buy: true,
        order_kind: ExchangeOrderKind::Limit,
        price_source: PriceSource::LimitInput {
            value: "100.123456".to_string(),
            invalid_message: "Invalid price",
        },
        quantity_source: QuantitySource::UserInput {
            value: "250.5".to_string(),
            denomination: QuantityDenomination::UsdNotional,
            invalid_message: "Invalid quantity",
            precision_invalid_message: "Invalid quantity for asset precision",
        },
        reduce_only_source: ReduceOnlySource::Form(true),
    }
}

fn market_usd_intent(
    surface: OrderSurface,
    reference: MarketUsdSizeReference,
    is_buy: bool,
) -> PlaceIntent {
    PlaceIntent {
        surface,
        symbol_key: "BTC".to_string(),
        is_buy,
        order_kind: ExchangeOrderKind::Market,
        price_source: PriceSource::MarketWithSlippage {
            invalid_message: Some("Invalid market price"),
            usd_size_reference: reference,
        },
        quantity_source: QuantitySource::UserInput {
            value: "250".to_string(),
            denomination: QuantityDenomination::UsdNotional,
            invalid_message: "Invalid quantity",
            precision_invalid_message: "Invalid quantity for asset precision",
        },
        reduce_only_source: ReduceOnlySource::Form(false),
    }
}

fn move_modify_intent(symbol_key: &str) -> ModifyIntent<'_> {
    ModifyIntent {
        surface: OrderSurface::Move,
        symbol_key,
        oid: 42,
        is_buy: true,
        new_price: 101.0,
        original_price: "100",
        size: "0.25",
        invalid_size_message: "Move failed: open order has invalid size",
        reduce_only: Some(false),
        reduce_only_missing_message: concat!(
            "Move failed: open order reduce-only metadata is unavailable; ",
            "refresh account data before moving this order"
        ),
        invalid_price_message: "Move failed: open order has invalid price",
    }
}

fn purr_spot_symbol() -> ExchangeSymbol {
    ExchangeSymbol {
        ticker: "PURR".to_string(),
        category: "spot".to_string(),
        display_name: Some("PURR/USDC".to_string()),
        asset_index: 10_000,
        ..symbol("PURR/USDC", MarketType::Spot)
    }
}
