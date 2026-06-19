use super::{
    AccountData, AccountDataCompleteness, AccountDataFetchScope, AccountDataSection,
    parse_account_number,
};
use crate::account::types::{
    AccountAbstractionMode, AssetPosition, ClearinghouseState, FundingDelta, FundingEntry,
    MarginSummary, OpenOrder, Position, PositionLeverage, SpotBalance, SpotClearinghouseState,
    UserFill,
};
use crate::api::{ExchangeSymbol, MarketType};

mod completeness;
mod fetch_scope;
mod freshness;
mod margin;
mod parsing;

fn account_data_for_available_margin(
    abstraction: AccountAbstractionMode,
    portfolio_margin_enabled: bool,
) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        fetched_at_ms: 1_000,
        account_abstraction: abstraction,
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "100".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "25".to_string(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: std::collections::HashMap::new(),
        spot: SpotClearinghouseState {
            balances: vec![
                SpotBalance {
                    coin: "USDC".to_string(),
                    token: Some(0),
                    total: "90".to_string(),
                    hold: "10".to_string(),
                    entry_ntl: "0".to_string(),
                    supplied: None,
                },
                SpotBalance {
                    coin: "USDH".to_string(),
                    token: Some(360),
                    total: "30".to_string(),
                    hold: "5".to_string(),
                    entry_ntl: "0".to_string(),
                    supplied: None,
                },
            ],
            portfolio_margin_enabled,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: Some(vec![
                (0, "55".to_string()),
                (360, "22".to_string()),
            ]),
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: Default::default(),
    }
}

fn account_data_snapshot(fetched_at_ms: u64) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        fetched_at_ms,
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
        fee_rates: Default::default(),
        completeness: Default::default(),
    }
}

#[test]
fn account_data_debug_summarizes_account_payloads() {
    let mut data = account_data_snapshot(123);
    data.clearinghouse.margin_summary.account_value = "account-secret-equity".to_string();
    data.spot.balances.push(SpotBalance {
        coin: "SECRETCOIN".to_string(),
        token: Some(1),
        total: "balance-secret-total".to_string(),
        hold: "balance-secret-hold".to_string(),
        entry_ntl: "0".to_string(),
        supplied: None,
    });
    data.open_orders.push(OpenOrder {
        coin: "SECRETORDER".to_string(),
        side: "B".to_string(),
        limit_px: "order-secret-price".to_string(),
        sz: "1".to_string(),
        oid: 42,
        timestamp: 7,
        reduce_only: None,
        is_trigger: None,
        order_type: None,
        tif: None,
        trigger_px: None,
    });
    data.fills.push(UserFill {
        coin: "SECRETFILL".to_string(),
        px: "fill-secret-price".to_string(),
        sz: "1".to_string(),
        side: "B".to_string(),
        time: 8,
        hash: Some("fill-secret-hash".to_string()),
        tid: Some(9),
        oid: Some(10),
        dir: "Open Long".to_string(),
        closed_pnl: "fill-secret-pnl".to_string(),
        fee: "fill-secret-fee".to_string(),
    });
    data.funding_history.push(FundingEntry {
        delta: FundingDelta {
            coin: "SECRETFUND".to_string(),
            funding_rate: "funding-secret-rate".to_string(),
            szi: "1".to_string(),
            usdc: "funding-secret-usdc".to_string(),
        },
        time: 11,
    });
    data.fee_rates.user_cross_rate = "fee-secret-rate".to_string();
    data.completeness
        .mark_incomplete(AccountDataSection::Positions, "warning-secret");

    let rendered = format!("{data:?}");

    assert!(rendered.contains("clearinghouse: positions_len=0"));
    assert!(rendered.contains("spot: balances_len=1"));
    assert!(rendered.contains("open_orders: len=1"));
    assert!(rendered.contains("fills: len=1"));
    assert!(rendered.contains("funding_history: len=1"));
    assert!(rendered.contains("fee_rates: <redacted>"));
    assert!(rendered.contains("positions_complete: false"));
    for secret in [
        "account-secret-equity",
        "SECRETCOIN",
        "balance-secret-total",
        "SECRETORDER",
        "order-secret-price",
        "SECRETFILL",
        "fill-secret-hash",
        "fill-secret-pnl",
        "SECRETFUND",
        "funding-secret-rate",
        "fee-secret-rate",
        "warning-secret",
    ] {
        assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
    }
}

fn perp_symbol(key: &str, max_leverage: u32) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.split(':').nth(1).unwrap_or(key).to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn asset_position(coin: &str, leverage_type: &str, leverage: u32) -> AssetPosition {
    AssetPosition {
        position: Position {
            coin: coin.to_string(),
            szi: "1".to_string(),
            entry_px: "100".to_string(),
            position_value: "100".to_string(),
            unrealized_pnl: "0".to_string(),
            liquidation_px: None,
            leverage: PositionLeverage {
                leverage_type: leverage_type.to_string(),
                value: leverage,
            },
            margin_used: "10".to_string(),
            cum_funding: None,
        },
        liquidation_px: None,
    }
}

#[test]
fn leverage_lookup_for_hip3_symbol_does_not_suffix_match_main_position() {
    let mut data = account_data_snapshot(1_000);
    data.clearinghouse.asset_positions = vec![asset_position("BTC", "cross", 20)];
    let symbols = vec![perp_symbol("xyz:BTC", 7)];

    assert_eq!(
        data.get_leverage_for("xyz:BTC", &symbols),
        Some((true, 7, false))
    );
}

#[test]
fn leverage_lookup_for_hip3_symbol_uses_exact_position() {
    let mut data = account_data_snapshot(1_000);
    data.clearinghouse.asset_positions = vec![
        asset_position("BTC", "cross", 20),
        asset_position("xyz:BTC", "isolated", 4),
    ];
    let symbols = vec![perp_symbol("xyz:BTC", 7)];

    assert_eq!(
        data.get_leverage_for("xyz:BTC", &symbols),
        Some((false, 4, true))
    );
}
