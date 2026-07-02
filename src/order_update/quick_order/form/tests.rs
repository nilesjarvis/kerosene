use super::{quick_order_quantity_for_percentage, toggled_quick_order_quantity_text};
use crate::account::{
    AccountAbstractionMode, AccountData, AccountDataCompleteness, AssetPosition,
    ClearinghouseState, MarginSummary, Position, PositionLeverage, SpotBalance,
    SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartInstance;
use crate::order_execution::QuickOrderForm;
use crate::timeframe::Timeframe;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

fn symbol(key: &str) -> ExchangeSymbol {
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

fn account_data_without_positions() -> AccountData {
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

fn spot_symbol(key: &str, ticker: &str, sz_decimals: u32) -> ExchangeSymbol {
    ExchangeSymbol {
        ticker: ticker.to_string(),
        category: "spot".to_string(),
        display_name: Some(format!("{ticker}/USDC")),
        sz_decimals,
        market_type: MarketType::Spot,
        ..symbol(key)
    }
}

fn spot_balance(coin: &str, token: Option<u32>, total: &str, hold: &str) -> SpotBalance {
    SpotBalance {
        coin: coin.to_string(),
        token,
        total: total.to_string(),
        hold: hold.to_string(),
        entry_ntl: "0".to_string(),
        supplied: None,
    }
}

fn quick_order_form(quantity_is_usd: bool) -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: String::new(),
        quantity_is_usd,
        percentage: 0.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 0.0,
        click_y: 0.0,
        chart_w: 400.0,
        chart_h: 240.0,
    }
}

fn spot_quick_order_terminal(
    chart_id: u64,
    quantity_is_usd: bool,
    abstraction: AccountAbstractionMode,
    withdrawable: &str,
    balances: Vec<SpotBalance>,
) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![spot_symbol("@107", "HYPE", 2)];
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    let mut data = account_data_without_positions();
    data.account_abstraction = abstraction;
    data.clearinghouse.withdrawable = withdrawable.to_string();
    data.spot.balances = balances;
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);
    let mut instance = ChartInstance::new(chart_id, "@107".to_string(), Timeframe::H1);
    instance.set_quick_order(quick_order_form(quantity_is_usd));
    terminal.charts.insert(chart_id, instance);
    terminal
}

fn account_data_with_position(coin: &str, szi: &str) -> AccountData {
    let mut data = account_data_without_positions();
    data.clearinghouse.asset_positions = vec![AssetPosition {
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
    }];
    data
}

#[test]
fn quick_order_percentage_quantity_formats_usd_and_coin() {
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 1_000.0, true, Some(100.0), 4),
        "250.00"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 1_000.0, false, Some(100.0), 4),
        "2.5000"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 5_000_000.0, false, Some(100.0), 2),
        "12,500.00"
    );
}

#[test]
fn quick_order_percentage_quantity_rejects_invalid_inputs() {
    assert_eq!(
        quick_order_quantity_for_percentage(f32::NAN, 1_000.0, true, Some(100.0), 4),
        "0"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 0.0, true, Some(100.0), 4),
        "0"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 1_000.0, false, None, 4),
        "0"
    );
}

#[test]
fn toggled_quick_order_quantity_converts_when_reference_price_is_available() {
    assert_eq!(
        toggled_quick_order_quantity_text("2.5", true, Some(100.0), 4),
        "250.00"
    );
    assert_eq!(
        toggled_quick_order_quantity_text("250", false, Some(100.0), 4),
        "2.5000"
    );
    assert_eq!(
        toggled_quick_order_quantity_text("1,234,500.00", false, Some(100.0), 2),
        "12,345.00"
    );
}

#[test]
fn quick_order_max_notional_uses_one_x_when_leverage_is_only_symbol_limit() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("BTC")];
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, account_data_without_positions());

    assert_eq!(terminal.quick_order_max_notional("BTC"), Some(1_000.0));
}

#[test]
fn quick_order_max_notional_ignores_stale_account_snapshot() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("BTC")];
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(OTHER_ACCOUNT, account_data_without_positions());

    assert_eq!(terminal.quick_order_max_notional("BTC"), None);
}

#[test]
fn quick_order_toggle_denomination_does_not_change_main_ticket_denomination() {
    let chart_id = 7;
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("BTC")];
    terminal.order_quantity_is_usd = false;
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.set_quick_order(QuickOrderForm {
        price: 100.0,
        quantity: "1".to_string(),
        quantity_is_usd: false,
        percentage: 0.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 0.0,
        click_y: 0.0,
        chart_w: 400.0,
        chart_h: 240.0,
    });
    terminal.charts.insert(chart_id, instance);

    terminal.handle_quick_order_toggle_denomination(chart_id);

    let form = terminal
        .charts
        .get(&chart_id)
        .and_then(|instance| instance.quick_order.as_ref())
        .expect("quick-order form should still be open");
    assert!(form.quantity_is_usd);
    assert_eq!(form.quantity, "100.00");
    assert!(!terminal.order_quantity_is_usd);
}

#[test]
fn quick_order_percentage_records_source_and_manual_edit_clears_it() {
    let chart_id = 7;
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("BTC")];
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, account_data_without_positions());
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.set_quick_order(QuickOrderForm {
        price: 100.0,
        quantity: String::new(),
        quantity_is_usd: false,
        percentage: 0.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 0.0,
        click_y: 0.0,
        chart_w: 400.0,
        chart_h: 240.0,
    });
    terminal.charts.insert(chart_id, instance);

    terminal.handle_quick_order_percentage_changed(chart_id, 25.0);

    let form = terminal
        .charts
        .get(&chart_id)
        .and_then(|instance| instance.quick_order.as_ref())
        .expect("quick-order form should still be open");
    let provenance = form
        .quantity_provenance
        .as_ref()
        .expect("percentage-derived quantity should have provenance");
    assert_eq!(provenance.account_address, TEST_ACCOUNT);
    assert_eq!(
        provenance.account_data_revision,
        terminal.account_data_revision
    );
    assert_eq!(provenance.symbol_key, "BTC");
    assert!(!provenance.quantity_is_usd);
    assert_eq!(provenance.reference_price, Some(100.0));

    terminal.handle_quick_order_qty_changed(chart_id, "1.25".to_string());

    let form = terminal
        .charts
        .get(&chart_id)
        .and_then(|instance| instance.quick_order.as_ref())
        .expect("quick-order form should still be open");
    assert_eq!(form.quantity, "1.25");
    assert!(form.quantity_provenance.is_none());
}

#[test]
fn reduce_only_quick_order_percentage_sizes_coin_quantity_from_position() {
    let chart_id = 7;
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("BTC")];
    terminal.order_reduce_only = true;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_position("BTC", "2"),
    );
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.set_quick_order(QuickOrderForm {
        price: 100.0,
        quantity: String::new(),
        quantity_is_usd: false,
        percentage: 0.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 0.0,
        click_y: 0.0,
        chart_w: 400.0,
        chart_h: 240.0,
    });
    terminal.charts.insert(chart_id, instance);

    terminal.handle_quick_order_percentage_changed(chart_id, 25.0);

    let form = terminal
        .charts
        .get(&chart_id)
        .and_then(|instance| instance.quick_order.as_ref())
        .expect("quick-order form should still be open");
    assert_eq!(form.quantity, "0.50000");
    assert!(
        form.quantity_provenance
            .as_ref()
            .is_some_and(|provenance| provenance.reduce_only)
    );
}

#[test]
fn reduce_only_quick_order_percentage_sizes_usd_quantity_from_position_notional() {
    let chart_id = 7;
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("BTC")];
    terminal.order_reduce_only = true;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_position("BTC", "2"),
    );
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.set_quick_order(QuickOrderForm {
        price: 100.0,
        quantity: String::new(),
        quantity_is_usd: true,
        percentage: 0.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 0.0,
        click_y: 0.0,
        chart_w: 400.0,
        chart_h: 240.0,
    });
    terminal.charts.insert(chart_id, instance);

    terminal.handle_quick_order_percentage_changed(chart_id, 25.0);

    let form = terminal
        .charts
        .get(&chart_id)
        .and_then(|instance| instance.quick_order.as_ref())
        .expect("quick-order form should still be open");
    assert_eq!(form.quantity, "50.00");
    assert!(
        form.quantity_provenance
            .as_ref()
            .is_some_and(|provenance| provenance.reduce_only)
    );
}

#[test]
fn reduce_only_quick_order_percentage_without_position_fails_closed() {
    let chart_id = 7;
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("BTC")];
    terminal.order_reduce_only = true;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, account_data_without_positions());
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.set_quick_order(QuickOrderForm {
        price: 100.0,
        quantity: "stale".to_string(),
        quantity_is_usd: false,
        percentage: 0.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 0.0,
        click_y: 0.0,
        chart_w: 400.0,
        chart_h: 240.0,
    });
    terminal.charts.insert(chart_id, instance);

    terminal.handle_quick_order_percentage_changed(chart_id, 25.0);

    let form = terminal
        .charts
        .get(&chart_id)
        .and_then(|instance| instance.quick_order.as_ref())
        .expect("quick-order form should still be open");
    assert_eq!(form.quantity, "0");
    assert!(form.quantity_provenance.is_none());
}

#[test]
fn spot_quick_order_percentage_sizes_sell_all_from_sellable_base_balance() {
    let chart_id = 7;
    let mut terminal = spot_quick_order_terminal(
        chart_id,
        false,
        AccountAbstractionMode::Default,
        "50000",
        vec![
            spot_balance("HYPE", Some(107), "100", "0"),
            spot_balance("USDC", Some(0), "50", "0"),
        ],
    );

    terminal.handle_quick_order_percentage_changed(chart_id, 100.0);

    let form = terminal
        .charts
        .get(&chart_id)
        .and_then(|instance| instance.quick_order.as_ref())
        .expect("quick-order form should still be open");
    assert_eq!(form.quantity, "100.00");
    assert!(form.quantity_provenance.is_some());
}

#[test]
fn spot_quick_order_percentage_respects_holds_on_base_balance() {
    let chart_id = 7;
    let mut terminal = spot_quick_order_terminal(
        chart_id,
        false,
        AccountAbstractionMode::Default,
        "50000",
        vec![spot_balance("HYPE", Some(107), "100", "25")],
    );

    terminal.handle_quick_order_percentage_changed(chart_id, 100.0);

    let form = terminal
        .charts
        .get(&chart_id)
        .and_then(|instance| instance.quick_order.as_ref())
        .expect("quick-order form should still be open");
    assert_eq!(form.quantity, "75.00");
}

#[test]
fn spot_quick_order_percentage_uses_spot_usdc_for_spot_only_disabled_account() {
    let chart_id = 7;
    let mut terminal = spot_quick_order_terminal(
        chart_id,
        true,
        AccountAbstractionMode::Disabled,
        "0",
        vec![spot_balance("USDC", Some(0), "10000", "0")],
    );

    terminal.handle_quick_order_percentage_changed(chart_id, 50.0);

    let form = terminal
        .charts
        .get(&chart_id)
        .and_then(|instance| instance.quick_order.as_ref())
        .expect("quick-order form should still be open");
    assert_eq!(form.quantity, "5,000.00");
}

#[test]
fn perp_quick_order_percentage_still_sizes_from_margin_with_spot_balances_present() {
    let chart_id = 7;
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("BTC")];
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    let mut data = account_data_without_positions();
    data.spot.balances = vec![spot_balance("USDC", Some(0), "2000", "0")];
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.set_quick_order(quick_order_form(true));
    terminal.charts.insert(chart_id, instance);

    terminal.handle_quick_order_percentage_changed(chart_id, 100.0);

    let form = terminal
        .charts
        .get(&chart_id)
        .and_then(|instance| instance.quick_order.as_ref())
        .expect("quick-order form should still be open");
    assert_eq!(form.quantity, "2,000.00");
}
