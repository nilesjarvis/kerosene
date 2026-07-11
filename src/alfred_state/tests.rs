use super::{AlfredCommandId, AlfredCommandKind};
use crate::account::{
    AccountData, AccountDataCompleteness, AccountDataSection, AssetPosition, ClearinghouseState,
    MarginSummary, Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::config::AccountProfile;
use crate::order_execution::PendingOrderAction;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

fn connect_test_account(terminal: &mut TradingTerminal) {
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.wallet_address_input = TEST_ACCOUNT.to_string();
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: TEST_ACCOUNT.to_string(),
        agent_key: String::new().into(),
        hydromancer_api_key: String::new().into(),
    }];
    terminal.active_account_index = 0;
}

fn connect_ready_test_account(terminal: &mut TradingTerminal) {
    connect_test_account(terminal);
    terminal.set_committed_agent_key_for_test("committed-agent-key");
}

fn exchange_symbol(key: &str) -> ExchangeSymbol {
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
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn active_position(coin: &str) -> AssetPosition {
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

fn account_data_with_positions(positions: Vec<&str>, fetched_at_ms: u64) -> AccountData {
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
            asset_positions: positions.into_iter().map(active_position).collect(),
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

fn add_mid(terminal: &mut TradingTerminal, coin: &str, mid: f64) {
    terminal.all_mids.insert(coin.to_string(), mid);
    terminal
        .all_mids_updated_at_ms
        .insert(coin.to_string(), TradingTerminal::now_ms());
}

fn nuke_command_or_panic(terminal: &TradingTerminal) -> super::AlfredCommand {
    let commands = terminal.alfred_filtered_commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NukePositions);
    commands.into_iter().next().expect("nuke command")
}

#[test]
fn alfred_defaults_to_add_widget_commands() {
    let terminal = TradingTerminal::boot().0;
    let commands = terminal.alfred_filtered_commands();

    assert!(
        commands
            .iter()
            .any(|command| command.id == AlfredCommandId::AddCandlestickChart)
    );
    assert!(
        commands
            .iter()
            .all(|command| command.kind != AlfredCommandKind::Trading)
    );
}

#[test]
fn alfred_catalog_includes_session_data_widget() {
    let terminal = TradingTerminal::boot().0;

    let command = terminal
        .alfred_filtered_commands()
        .into_iter()
        .find(|command| command.id == AlfredCommandId::AddSessionDataPane)
        .expect("Session Data should be an Alfred add-widget command");

    assert_eq!(command.title, "Session Data");
    assert_eq!(command.kind, AlfredCommandKind::AddWidget);
    assert!(command.message.is_some());
}

#[test]
fn alfred_catalog_includes_positions_history_widget() {
    let terminal = TradingTerminal::boot().0;

    let command = terminal
        .alfred_filtered_commands()
        .into_iter()
        .find(|command| command.id == AlfredCommandId::AddPositionsHistoryPane)
        .expect("Positions / History should be an Alfred add-widget command");

    assert_eq!(command.title, "Positions / History");
    assert_eq!(command.kind, AlfredCommandKind::AddWidget);
    assert!(command.message.is_some());
}

#[test]
fn alfred_shows_only_trade_draft_for_trade_queries() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "buy btc".to_string();

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NaturalLanguageTrading);
}

#[test]
fn alfred_shows_only_trade_draft_for_chase_queries() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "chase 1k HYPE".to_string();

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NaturalLanguageTrading);
}

#[test]
fn alfred_shows_only_nuke_command_for_nuke_query() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NukePositions);
}

#[test]
fn alfred_treats_close_all_as_nuke_command() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "close all".to_string();

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NukePositions);
}

#[test]
fn alfred_nuke_uses_committed_agent_key_gate() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();
    connect_test_account(&mut terminal);
    terminal.wallet_key_input = "draft-agent-key".to_string().into();

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NukePositions);
    assert!(!commands[0].enabled);
    assert_eq!(
        commands[0].disabled_reason.as_deref(),
        Some("Connect wallet and enter agent key first")
    );

    terminal.set_committed_agent_key_for_test("committed-agent-key");
    terminal.wallet_key_input = "draft-agent-key".to_string().into();
    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NukePositions);
    assert!(!commands[0].enabled);
    assert_eq!(
        commands[0].disabled_reason.as_deref(),
        Some("No account data available")
    );
}

#[test]
fn alfred_nuke_disabled_while_account_reconciliation_required() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("committed-agent-key");
    terminal.account_reconciliation_required = true;

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NukePositions);
    assert!(!commands[0].enabled);
    assert_eq!(
        commands[0].disabled_reason.as_deref(),
        Some("Account refresh pending; wait for fresh account data before NUKE")
    );
}

#[test]
fn alfred_nuke_disabled_while_trading_request_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();
    connect_ready_test_account(&mut terminal);
    terminal.pending_order_action = Some(PendingOrderAction::Buy);

    let command = nuke_command_or_panic(&terminal);

    assert!(!command.enabled);
    assert_eq!(
        command.disabled_reason.as_deref(),
        Some("Wait for pending trading requests to finish before NUKE")
    );
}

#[test]
fn alfred_nuke_disabled_for_active_wallet_mismatch() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();
    connect_ready_test_account(&mut terminal);
    terminal.wallet_address_input = OTHER_ACCOUNT.to_string();

    let command = nuke_command_or_panic(&terminal);

    assert!(!command.enabled);
    assert_eq!(
        command.disabled_reason.as_deref(),
        Some("Connected wallet no longer matches the active account; reconnect before trading")
    );
}

#[test]
fn alfred_nuke_disabled_for_stale_account_snapshot() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();
    connect_ready_test_account(&mut terminal);
    terminal.exchange_symbols = vec![exchange_symbol("BTC")];
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_positions(vec!["BTC"], 1),
    );
    add_mid(&mut terminal, "BTC", 100.0);

    let command = nuke_command_or_panic(&terminal);

    assert!(!command.enabled);
    let reason = command.disabled_reason.as_deref().expect("disabled reason");
    assert!(reason.contains("Account data is stale"));
    assert!(reason.contains("refresh before NUKE"));
}

#[test]
fn alfred_nuke_disabled_for_incomplete_position_snapshot() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();
    connect_ready_test_account(&mut terminal);
    terminal.exchange_symbols = vec![exchange_symbol("BTC")];
    let mut data = account_data_with_positions(vec!["BTC"], TradingTerminal::now_ms());
    data.completeness
        .mark_incomplete(AccountDataSection::Positions, "HIP-3 positions unavailable");
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);
    add_mid(&mut terminal, "BTC", 100.0);

    let command = nuke_command_or_panic(&terminal);

    assert!(!command.enabled);
    assert_eq!(
        command.disabled_reason.as_deref(),
        Some("Positions may be incomplete: HIP-3 positions unavailable; refresh before NUKE")
    );
}

#[test]
fn alfred_nuke_enabled_for_degraded_fallback_snapshot() {
    // Regression: a complete Hyperliquid fallback snapshot (degraded, but with
    // usable positions) must keep the NUKE command enabled rather than locking
    // out a Hydromancer-without-key config.
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();
    connect_ready_test_account(&mut terminal);
    terminal.exchange_symbols = vec![exchange_symbol("BTC")];
    let mut data = account_data_with_positions(vec!["BTC"], TradingTerminal::now_ms());
    data.completeness.mark_degraded(
        AccountDataSection::Positions,
        "Hydromancer API key missing; used Hyperliquid fallback",
    );
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);
    add_mid(&mut terminal, "BTC", 100.0);

    let command = nuke_command_or_panic(&terminal);

    assert!(command.enabled);
    assert_eq!(command.title, "NUKE 1 position");
}

#[test]
fn alfred_nuke_disabled_for_hidden_unrouteable_exposure() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();
    connect_ready_test_account(&mut terminal);
    terminal.exchange_symbols = vec![exchange_symbol("BTC"), exchange_symbol("HIDDEN")];
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_positions(vec!["BTC", "HIDDEN"], TradingTerminal::now_ms()),
    );
    terminal.muted_tickers.insert("HIDDEN".to_string());
    add_mid(&mut terminal, "BTC", 100.0);

    let command = nuke_command_or_panic(&terminal);

    assert!(!command.enabled);
    assert_eq!(
        command.disabled_reason.as_deref(),
        Some("Cannot NUKE: hidden exposure unresolvable: HIDDEN (no mid price)")
    );
}

#[test]
fn alfred_nuke_disabled_when_all_visible_positions_are_unrouteable() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();
    connect_ready_test_account(&mut terminal);
    terminal.exchange_symbols = vec![exchange_symbol("BTC")];
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_positions(vec!["BTC"], TradingTerminal::now_ms()),
    );

    let command = nuke_command_or_panic(&terminal);

    assert!(!command.enabled);
    assert_eq!(
        command.disabled_reason.as_deref(),
        Some("Cannot NUKE: BTC (no mid price)")
    );
}

#[test]
fn alfred_nuke_enabled_with_visible_partial_skip_warning() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();
    connect_ready_test_account(&mut terminal);
    terminal.exchange_symbols = vec![exchange_symbol("BTC"), exchange_symbol("ETH")];
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_positions(vec!["BTC", "ETH"], TradingTerminal::now_ms()),
    );
    add_mid(&mut terminal, "BTC", 100.0);

    let command = nuke_command_or_panic(&terminal);

    assert!(command.enabled);
    assert_eq!(command.title, "NUKE 1 position");
    assert_eq!(
        command.detail,
        "Market close: BTC; skipping ETH (no mid price)"
    );
}

#[test]
fn alfred_nuke_enabled_for_fresh_routeable_plan() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();
    connect_ready_test_account(&mut terminal);
    terminal.exchange_symbols = vec![exchange_symbol("BTC")];
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_positions(vec!["BTC"], TradingTerminal::now_ms()),
    );
    add_mid(&mut terminal, "BTC", 100.0);

    let command = nuke_command_or_panic(&terminal);

    assert!(command.enabled);
    assert!(matches!(
        command.message,
        Some(crate::message::Message::AlfredSubmit)
    ));
    assert_eq!(command.title, "NUKE 1 position");
}

#[test]
fn alfred_shows_only_close_position_command_for_close_queries() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "close HYPE".to_string();

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::ClosePosition);
}

#[test]
fn alfred_state_and_command_debug_redact_order_text_without_changing_it() {
    let state = super::AlfredState {
        open: true,
        query: "buy 12345.6789 private-alfred-symbol-sentinel at 98765.4321".to_string(),
        selected_index: 3,
    };
    let command = super::AlfredCommand::new(
        AlfredCommandId::NaturalLanguageTrading,
        "title",
        "detail",
        "tag",
        AlfredCommandKind::Trading,
        Some(crate::message::Message::AlfredSubmit),
        &["trade"],
    )
    .with_dynamic_text(
        "private-alfred-title-sentinel".to_string(),
        "private-alfred-detail-sentinel".to_string(),
        "private-alfred-tag-sentinel".to_string(),
    );

    let rendered = format!("{state:?} {command:?}");

    assert!(rendered.contains("has_query: true"), "{rendered}");
    assert!(rendered.contains("has_message: true"), "{rendered}");
    for sensitive in [
        "12345.6789",
        "98765.4321",
        "private-alfred-symbol-sentinel",
        "private-alfred-title-sentinel",
        "private-alfred-detail-sentinel",
        "private-alfred-tag-sentinel",
    ] {
        assert!(!rendered.contains(sensitive), "{rendered}");
    }
    assert!(state.query.contains("private-alfred-symbol-sentinel"));
    assert_eq!(command.title, "private-alfred-title-sentinel");
    assert_eq!(command.detail, "private-alfred-detail-sentinel");
    assert_eq!(command.tag, "private-alfred-tag-sentinel");
}
