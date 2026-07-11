use super::*;
use crate::account::{
    AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
    PositionLeverage, SpotBalance, SpotClearinghouseState, UserFeeRates,
};
use crate::api::ExchangeSymbol;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

fn perp_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 50,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn spot_symbol(key: &str, ticker: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "spot".to_string(),
        display_name: Some(format!("{ticker}/USDC")),
        keywords: vec!["spot".to_string()],
        asset_index: 10_107,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
    }
}

fn spot_balance(coin: &str, total: &str, hold: &str) -> SpotBalance {
    SpotBalance {
        coin: coin.to_string(),
        token: None,
        total: total.to_string(),
        hold: hold.to_string(),
        entry_ntl: "100".to_string(),
        supplied: None,
    }
}

fn perp_position(coin: &str) -> AssetPosition {
    AssetPosition {
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
    }
}

fn account_data(positions: Vec<AssetPosition>, balances: Vec<SpotBalance>) -> AccountData {
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
            balances,
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

fn close_terminal(positions: Vec<AssetPosition>, balances: Vec<SpotBalance>) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![perp_symbol("HYPE"), spot_symbol("@107", "HYPE")];
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, account_data(positions, balances));
    terminal.account_loading = false;
    terminal
}

fn close_draft_or_panic(terminal: &TradingTerminal, query: &str) -> AlfredClosePositionDraft {
    match terminal.alfred_close_position_draft(query) {
        Some(draft) => draft,
        None => panic!("missing close draft for {query}"),
    }
}

#[test]
fn close_spot_holding_reports_spot_balance_instead_of_no_position() {
    let terminal = close_terminal(Vec::new(), vec![spot_balance("HYPE", "25", "5")]);

    let draft = close_draft_or_panic(&terminal, "close hype");

    assert!(!draft.can_submit());
    assert_eq!(draft.coin, None);
    assert_eq!(
        draft.error.as_deref(),
        Some("HYPE is a spot balance; close only closes perp positions — try 'sell 20 HYPE/USDC'")
    );
}

#[test]
fn close_spot_holding_by_pair_key_reports_spot_balance() {
    let terminal = close_terminal(Vec::new(), vec![spot_balance("HYPE", "25", "25")]);

    let draft = close_draft_or_panic(&terminal, "close @107");

    assert!(!draft.can_submit());
    // Fully on hold: no sell suggestion, but the limitation is still explicit.
    assert_eq!(
        draft.error.as_deref(),
        Some("HYPE is a spot balance; close only closes perp positions")
    );
}

#[test]
fn close_without_position_or_spot_balance_keeps_no_position_error() {
    let terminal = close_terminal(Vec::new(), Vec::new());

    let draft = close_draft_or_panic(&terminal, "close hype");

    assert!(!draft.can_submit());
    assert_eq!(draft.error.as_deref(), Some("No open position for HYPE"));
}

#[test]
fn close_prefers_perp_position_when_spot_balance_also_exists() {
    let terminal = close_terminal(
        vec![perp_position("HYPE")],
        vec![spot_balance("HYPE", "25", "0")],
    );

    let draft = close_draft_or_panic(&terminal, "close hype");

    assert!(draft.can_submit());
    assert_eq!(draft.coin.as_deref(), Some("HYPE"));
    assert_eq!(draft.error, None);
}

#[test]
fn parses_full_position_close() {
    let intent = parse_close_position_intent("close HYPE").expect("close intent");

    assert_eq!(intent.symbol.as_deref(), Some("HYPE"));
    assert_eq!(intent.fraction, None);
    assert_eq!(intent.error, None);
}

#[test]
fn parses_fractional_position_close() {
    let intent = parse_close_position_intent("close hype 25").expect("close intent");

    assert_eq!(intent.symbol.as_deref(), Some("hype"));
    assert_eq!(intent.fraction, Some(0.25));
    assert_eq!(intent.error, None);
}

#[test]
fn parses_percent_sign_position_close() {
    let intent = parse_close_position_intent("close hype 12.5%").expect("close intent");

    assert_eq!(intent.fraction, Some(0.125));
    assert_eq!(intent.error, None);
}

#[test]
fn parses_fraction_before_ticker_position_close() {
    let intent = parse_close_position_intent("close 100 hype").expect("close intent");

    assert_eq!(intent.symbol.as_deref(), Some("hype"));
    assert_eq!(intent.fraction, Some(1.0));
    assert_eq!(intent.error, None);
}

#[test]
fn parses_percent_sign_before_ticker_position_close() {
    let intent = parse_close_position_intent("close 100% HYPE").expect("close intent");

    assert_eq!(intent.symbol.as_deref(), Some("HYPE"));
    assert_eq!(intent.fraction, Some(1.0));
    assert_eq!(intent.error, None);
}

#[test]
fn rejects_invalid_close_percentages() {
    let intent = parse_close_position_intent("close hype 125").expect("close intent");

    assert_eq!(
        intent.error.as_deref(),
        Some("Use a close percentage from 1 to 100")
    );
}

#[test]
fn ignores_non_close_queries() {
    assert_eq!(parse_close_position_intent("buy HYPE"), None);
    assert_eq!(parse_close_position_intent("nuke"), None);
}

#[test]
fn close_draft_and_intent_debug_redact_order_values_without_changing_them() {
    let intent = ParsedClosePositionIntent {
        symbol: Some("private-close-symbol-sentinel".to_string()),
        fraction: Some(0.123_456_789),
        error: Some("private-close-error-sentinel".to_string()),
    };
    let draft = AlfredClosePositionDraft {
        coin: Some("private-close-coin-sentinel".to_string()),
        fraction: 0.987_654_321,
        title: "private-close-title-sentinel".to_string(),
        detail: "private-close-detail-sentinel".to_string(),
        tag: "private-close-tag-sentinel".to_string(),
        error: Some("private-close-draft-error-sentinel".to_string()),
    };

    let rendered = format!("{intent:?} {draft:?}");

    assert!(rendered.contains("<redacted>"), "{rendered}");
    for sensitive in [
        "private-close-symbol-sentinel",
        "private-close-error-sentinel",
        "private-close-coin-sentinel",
        "private-close-title-sentinel",
        "private-close-detail-sentinel",
        "private-close-tag-sentinel",
        "private-close-draft-error-sentinel",
        "0.123456789",
        "0.987654321",
    ] {
        assert!(!rendered.contains(sensitive), "{rendered}");
    }
    assert_eq!(
        intent.symbol.as_deref(),
        Some("private-close-symbol-sentinel")
    );
    assert_eq!(
        intent.fraction.map(f64::to_bits),
        Some(0.123_456_789_f64.to_bits())
    );
    assert_eq!(draft.coin.as_deref(), Some("private-close-coin-sentinel"));
    assert_eq!(draft.fraction.to_bits(), 0.987_654_321_f64.to_bits());
}
