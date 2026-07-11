use super::{OrderQuantityProvenance, OrderSizingBasis, TradingTerminal, position_size_for_symbol};
use crate::account::{
    AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
    Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
};
use crate::api::{BookLevel, ExchangeSymbol, MarketType, OrderBook};
use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};
use crate::signing::OrderKind;

mod order_book;
mod reduce_only;
mod reference_price;
mod spot;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
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
        market_type,
        outcome: None,
    }
}

fn book(best_bid: f64, best_ask: f64) -> OrderBook {
    OrderBook {
        bids: vec![BookLevel {
            px: best_bid,
            sz: 1.0,
        }],
        asks: vec![BookLevel {
            px: best_ask,
            sz: 1.0,
        }],
    }
}

fn terminal_with_order_book(mode: OrderBookSymbolMode) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.order_books.clear();
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.order_kind = OrderKind::Market;
    terminal.order_price.clear();
    terminal
        .order_books
        .insert(7, OrderBookInstance::new(7, mode, 1.0));
    terminal
}

fn set_order_book(terminal: &mut TradingTerminal, pane_id: u64, book: OrderBook) {
    let Some(instance) = terminal.order_books.get_mut(&pane_id) else {
        panic!("test order book");
    };
    instance.set_book(book);
}

fn account_data_with_positions(positions: Vec<AssetPosition>) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "1000".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "1000".to_string(),
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
        fetched_at_ms: TradingTerminal::now_ms(),
    }
}

fn asset_position(coin: &str, szi: &str) -> AssetPosition {
    AssetPosition {
        position: Position {
            coin: coin.to_string(),
            szi: szi.to_string(),
            entry_px: "100".to_string(),
            position_value: "0".to_string(),
            unrealized_pnl: "0".to_string(),
            liquidation_px: None,
            leverage: PositionLeverage {
                leverage_type: "cross".to_string(),
                value: 10,
            },
            margin_used: "0".to_string(),
            cum_funding: None,
        },
        liquidation_px: None,
    }
}

fn terminal_with_position(coin: &str, szi: &str) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.order_kind = OrderKind::Market;
    terminal.order_price.clear();
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_positions(vec![asset_position(coin, szi)]),
    );
    terminal
}

#[test]
fn margin_order_sizing_uses_one_x_when_leverage_is_only_symbol_limit() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    let data = account_data_with_positions(Vec::new());

    let Some(OrderSizingBasis::MarginNotional { max_notional }) =
        terminal.order_sizing_basis(&data)
    else {
        panic!("expected margin sizing basis");
    };

    assert_eq!(max_notional, 1_000.0);
}

#[test]
fn order_sizing_basis_debug_redacts_financial_values_without_changing_them() {
    const MARGIN_NOTIONAL: f64 = 98_765.432_1;
    const POSITION_SIZE: f64 = 12_345.678_9;
    const SELLABLE_SIZE: f64 = 45_678.912_3;
    let margin = OrderSizingBasis::MarginNotional {
        max_notional: MARGIN_NOTIONAL,
    };
    let reduce_only = OrderSizingBasis::ReduceOnlyPosition {
        position_size_coin: POSITION_SIZE,
    };
    let spot = OrderSizingBasis::SpotSellableBalance {
        sellable_size_coin: SELLABLE_SIZE,
    };

    let rendered = format!("{margin:?} {reduce_only:?} {spot:?}");

    assert!(rendered.contains("MarginNotional"), "{rendered}");
    assert!(rendered.contains("ReduceOnlyPosition"), "{rendered}");
    assert!(rendered.contains("SpotSellableBalance"), "{rendered}");
    for value in [MARGIN_NOTIONAL, POSITION_SIZE, SELLABLE_SIZE] {
        assert!(!rendered.contains(&format!("{value:?}")), "{rendered}");
    }

    let OrderSizingBasis::MarginNotional { max_notional } = margin else {
        panic!("expected margin basis");
    };
    let OrderSizingBasis::ReduceOnlyPosition { position_size_coin } = reduce_only else {
        panic!("expected reduce-only basis");
    };
    let OrderSizingBasis::SpotSellableBalance { sellable_size_coin } = spot else {
        panic!("expected spot basis");
    };
    assert_eq!(max_notional.to_bits(), MARGIN_NOTIONAL.to_bits());
    assert_eq!(position_size_coin.to_bits(), POSITION_SIZE.to_bits());
    assert_eq!(sellable_size_coin.to_bits(), SELLABLE_SIZE.to_bits());
}

#[test]
fn order_quantity_change_ignores_stale_account_snapshot_for_percentage() {
    let mut terminal = terminal_with_position("BTC", "0");
    terminal.account_data_address = Some(OTHER_ACCOUNT.to_string());
    terminal.order_quantity_is_usd = true;
    terminal.order_percentage = 75.0;

    terminal.handle_order_quantity_changed("500".to_string());

    assert_eq!(terminal.order_percentage, 0.0);
}

#[test]
fn order_percentage_change_ignores_stale_account_snapshot_for_quantity() {
    let mut terminal = terminal_with_position("BTC", "0");
    terminal.account_data_address = Some(OTHER_ACCOUNT.to_string());
    terminal.order_quantity_is_usd = true;
    terminal.order_quantity = "unchanged".to_string();

    terminal.handle_order_percentage_changed(50.0);

    assert_eq!(terminal.order_quantity, "unchanged");
}

#[test]
fn percentage_quantity_records_source_and_manual_edit_clears_it() {
    let mut terminal = terminal_with_position("BTC", "0");
    terminal.order_quantity_is_usd = true;

    terminal.handle_order_percentage_changed(50.0);

    assert!(terminal.order_quantity_provenance.is_some());

    terminal.handle_order_quantity_changed("500".to_string());

    assert!(terminal.order_quantity_provenance.is_none());
    assert_eq!(terminal.order_quantity, "500");
}

#[test]
fn order_quantity_provenance_debug_redacts_sizing_context_without_changing_it() {
    const SYMBOL: &str = "private-ticket-symbol-sentinel";
    const PERCENTAGE: f32 = 42.42;
    const REFERENCE_PRICE: f64 = 98_765.432_1;
    let provenance = OrderQuantityProvenance {
        account_address: TEST_ACCOUNT.to_string(),
        account_data_revision: 7,
        spot_balances_revision: 3,
        symbol_key: SYMBOL.to_string(),
        quantity_is_usd: true,
        percentage: PERCENTAGE,
        order_kind: OrderKind::Limit,
        reference_price: Some(REFERENCE_PRICE),
        reduce_only: true,
        market_universe: crate::config::MarketUniverseConfig::default(),
    };

    let rendered = format!("{provenance:?}");

    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(!rendered.contains(TEST_ACCOUNT), "{rendered}");
    assert!(!rendered.contains(SYMBOL), "{rendered}");
    assert!(!rendered.contains(&format!("{PERCENTAGE:?}")), "{rendered}");
    assert!(
        !rendered.contains(&format!("{REFERENCE_PRICE:?}")),
        "{rendered}"
    );
    assert!(rendered.contains("account_data_revision: 7"), "{rendered}");
    assert!(rendered.contains("spot_balances_revision: 3"), "{rendered}");
    assert!(rendered.contains("quantity_is_usd: true"), "{rendered}");
    assert!(rendered.contains("order_kind: Limit"), "{rendered}");
    assert!(rendered.contains("reduce_only: true"), "{rendered}");
    assert!(rendered.contains("market_universe: All"), "{rendered}");

    assert_eq!(provenance.account_address, TEST_ACCOUNT);
    assert_eq!(provenance.symbol_key, SYMBOL);
    assert_eq!(provenance.percentage.to_bits(), PERCENTAGE.to_bits());
    assert_eq!(
        provenance.reference_price.map(f64::to_bits),
        Some(REFERENCE_PRICE.to_bits())
    );
}

#[test]
fn percentage_quantity_recomputes_provenance_when_denomination_toggles() {
    let mut terminal = terminal_with_position("BTC", "0");
    terminal.order_quantity_is_usd = true;
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    terminal.handle_order_percentage_changed(25.0);
    assert!(
        terminal
            .order_quantity_provenance
            .as_ref()
            .is_some_and(|provenance| provenance.quantity_is_usd)
    );

    terminal.handle_toggle_order_denomination();

    assert!(!terminal.order_quantity_is_usd);
    assert_eq!(terminal.order_quantity, "25.00000");
    assert!(
        terminal
            .order_quantity_provenance
            .as_ref()
            .is_some_and(|provenance| !provenance.quantity_is_usd)
    );
}

#[test]
fn active_symbol_change_clears_percentage_quantity_provenance() {
    let mut terminal = terminal_with_position("BTC", "0");
    terminal
        .exchange_symbols
        .push(symbol("ETH", MarketType::Perp));
    terminal.order_quantity_is_usd = true;
    terminal.handle_order_percentage_changed(50.0);

    terminal.apply_active_symbol_selection("ETH".to_string(), "ETH".to_string());

    assert!(terminal.order_quantity.is_empty());
    assert_eq!(terminal.order_percentage, 0.0);
    assert!(terminal.order_quantity_provenance.is_none());
}

#[test]
fn percentage_quantity_reselection_without_account_snapshot_clears_stale_derived_quantity() {
    let mut terminal = terminal_with_position("BTC", "0");
    terminal.order_quantity_is_usd = true;
    terminal.handle_order_percentage_changed(50.0);

    assert!(!terminal.order_quantity.is_empty());
    assert!(terminal.order_quantity_provenance.is_some());

    terminal.account_data = None;
    terminal.account_data_address = None;
    terminal.bump_account_data_revision();
    terminal.handle_order_percentage_changed(25.0);

    assert!(terminal.order_quantity.is_empty());
    assert_eq!(terminal.order_percentage, 0.0);
    assert!(terminal.order_quantity_provenance.is_none());
}
