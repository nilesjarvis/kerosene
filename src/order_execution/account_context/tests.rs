use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, SpotClearinghouseState,
};
use crate::api::MarketType;
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config::AccountProfile;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

fn connect_test_account(terminal: &mut TradingTerminal) {
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.wallet_address_input = TEST_ACCOUNT.to_string();
    terminal.accounts = vec![AccountProfile {
        master_address: None,
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: TEST_ACCOUNT.to_string(),
        agent_key: sensitive_string("").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }];
    terminal.active_account_index = 0;
}

fn empty_account_data() -> AccountData {
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
        fee_rates: Default::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms: 1,
    }
}

#[test]
fn connected_order_account_address_rejects_missing_and_blank_values() {
    let mut terminal = TradingTerminal::boot().0;

    terminal.connected_address = None;
    assert_eq!(terminal.connected_order_account_address(), None);

    terminal.connected_address = Some(String::new());
    assert_eq!(terminal.connected_order_account_address(), None);

    terminal.connected_address = Some("   ".to_string());
    assert_eq!(terminal.connected_order_account_address(), None);
    assert!(!terminal.connected_order_account_matches("   "));
}

#[test]
fn connected_order_account_address_trims_surrounding_whitespace() {
    let mut terminal = TradingTerminal::boot().0;

    terminal.connected_address = Some(" 0xabc ".to_string());

    assert_eq!(
        terminal.connected_order_account_address(),
        Some("0xabc".to_string())
    );
    assert!(terminal.connected_order_account_matches("0xabc"));
    assert!(terminal.connected_order_account_matches(" 0xabc "));
    assert!(terminal.connected_order_account_matches(" 0XABC "));
}

#[test]
fn account_data_for_order_account_normalizes_case_and_whitespace() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_data_address = Some(" 0xAbC ".to_string());
    terminal.account_data = Some(empty_account_data());

    assert!(terminal.account_data_for_order_account(" 0xabc ").is_some());
    assert!(
        terminal
            .account_data_for_order_account_mut(" 0XABC ")
            .is_some()
    );
    assert!(terminal.account_data_for_order_account("0xdef").is_none());
}

#[test]
fn spot_exchange_dispatch_invalidates_only_the_connected_accounts_spot_balances() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.account_data_address = Some(TEST_ACCOUNT.to_string());
    let mut data = empty_account_data();
    data.completeness.spot_balances_complete = true;
    data.completeness.spot_balances_fetched_at_ms = Some(123);
    terminal.account_data = Some(data);
    let initial_revision = terminal.spot_balances_revision;

    terminal.invalidate_spot_balances_after_exchange_dispatch(TEST_ACCOUNT, MarketType::Spot);

    assert!(
        !terminal
            .account_data
            .as_ref()
            .expect("account data")
            .completeness
            .spot_balances_complete
    );
    assert_eq!(
        terminal.spot_balances_revision,
        initial_revision.wrapping_add(1)
    );

    terminal
        .account_data
        .as_mut()
        .expect("account data")
        .completeness
        .spot_balances_complete = true;
    let revision_after_spot = terminal.spot_balances_revision;
    terminal.invalidate_spot_balances_after_exchange_dispatch(TEST_ACCOUNT, MarketType::Perp);
    terminal.invalidate_spot_balances_after_exchange_dispatch(OTHER_ACCOUNT, MarketType::Spot);

    assert!(
        terminal
            .account_data
            .as_ref()
            .expect("account data")
            .completeness
            .spot_balances_complete
    );
    assert_eq!(terminal.spot_balances_revision, revision_after_spot);
}

#[test]
fn signing_guards_preserve_committed_key_and_error_priority() {
    fn assert_rejected(terminal: &mut TradingTerminal, key_present: bool, expected_status: &str) {
        assert_eq!(terminal.has_active_committed_agent_key(), key_present);
        assert!(terminal.captured_order_signing_context().is_none());
        assert_eq!(
            terminal.order_status.as_ref(),
            Some(&(expected_status.to_string(), true))
        );
    }

    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.wallet_key_input = sensitive_string("unsaved-draft-key");
    terminal.ghost_account_secret_ids.insert("acct-a".into());
    terminal.connected_address = None;
    terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
    let missing_key_or_connection = "Connect wallet and enter agent key first";

    for key in ["", " \t\n "] {
        terminal.accounts[0].agent_key = sensitive_string(key).into_zeroizing();
        assert_rejected(&mut terminal, false, missing_key_or_connection);
    }
    terminal.active_account_index = terminal.accounts.len();
    assert_rejected(&mut terminal, false, missing_key_or_connection);
    terminal.active_account_index = 0;

    terminal.accounts[0].agent_key = sensitive_string(" committed-key ").into_zeroizing();
    assert_rejected(
        &mut terminal,
        true,
        "Watch-only accounts cannot sign orders",
    );
    terminal.ghost_account_secret_ids.clear();
    assert_rejected(&mut terminal, true, missing_key_or_connection);
    terminal.connected_address = Some(" \t ".to_string());
    assert_rejected(&mut terminal, true, missing_key_or_connection);

    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    assert_rejected(
        &mut terminal,
        true,
        "Connected wallet no longer matches the active account; reconnect before trading",
    );
    terminal.wallet_address_input = TEST_ACCOUNT.to_string();
    terminal.order_status = Some(("previous status".to_string(), false));
    let (key, account) = terminal
        .captured_order_signing_context()
        .expect("matching committed context");
    assert_eq!(key.as_str(), "committed-key");
    assert_eq!(account, TEST_ACCOUNT);
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&("previous status".to_string(), false))
    );
}

#[test]
fn order_signing_context_rejects_active_wallet_mismatch() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
    terminal.accounts[0].wallet_address = OTHER_ACCOUNT.to_string();

    assert!(terminal.order_signing_context().is_none());
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&(
            "Connected wallet no longer matches the active account; reconnect before trading"
                .to_string(),
            true
        ))
    );
}

#[test]
fn order_signing_context_accepts_matching_active_wallet() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("  agent-key  ");

    let (key, account_address) = terminal
        .order_signing_context()
        .expect("matching context should trade");

    assert_eq!(key.as_str(), "agent-key");
    assert_eq!(account_address, TEST_ACCOUNT);
}

#[test]
fn checked_order_signing_account_rejects_active_wallet_mismatch() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
    terminal.accounts[0].wallet_address = OTHER_ACCOUNT.to_string();

    assert!(terminal.checked_order_signing_account().is_none());
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&(
            "Connected wallet no longer matches the active account; reconnect before trading"
                .to_string(),
            true
        ))
    );
}

#[test]
fn checked_order_signing_account_accepts_matching_active_wallet() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("  agent-key  ");

    let account_address = terminal
        .checked_order_signing_account()
        .expect("matching context should be available");

    assert_eq!(account_address, TEST_ACCOUNT);
}

#[test]
fn subaccount_signing_context_captures_child_target_and_committed_parent_key() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.accounts[0].master_address = Some(OTHER_ACCOUNT.to_string());
    terminal.set_committed_agent_key_for_test("parent-agent-key");
    terminal.wallet_key_input = sensitive_string("unsaved-draft-key");

    let (key, address) = terminal
        .order_signing_context()
        .expect("subaccount context");
    assert_eq!(address, TEST_ACCOUNT);
    assert_eq!(key.vault_address(), Some(TEST_ACCOUNT));
    assert_eq!(key.as_str(), "parent-agent-key");

    // In-flight work owns its target, even after the active profile changes.
    terminal.accounts[0].master_address = None;
    terminal.accounts[0].wallet_address = OTHER_ACCOUNT.to_string();
    terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
    terminal.connected_address = Some(OTHER_ACCOUNT.to_string());
    terminal.set_committed_agent_key_for_test("other-key");
    assert_eq!(key.clone_for_task().vault_address(), Some(TEST_ACCOUNT));
    assert_eq!(key.clone_for_task().as_str(), "parent-agent-key");
    let (master_key, _) = terminal.order_signing_context().expect("master context");
    assert_eq!(master_key.vault_address(), None);
}

#[test]
fn subaccount_signing_rejects_invalid_or_self_parent_binding() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("agent-key");
    for parent in ["", "malformed-parent", TEST_ACCOUNT] {
        terminal.accounts[0].master_address = Some(parent.to_string());
        assert!(terminal.order_signing_context().is_none());
        assert!(terminal.checked_order_signing_account().is_none());
        assert!(
            terminal
                .order_status
                .as_ref()
                .is_some_and(|(_, error)| *error)
        );
    }
}

#[test]
fn subaccount_signing_and_snapshots_reject_parent_account_context() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.accounts[0].master_address = Some(OTHER_ACCOUNT.to_string());
    terminal.set_committed_agent_key_for_test("parent-agent-key");
    terminal.connected_address = Some(OTHER_ACCOUNT.to_string());
    assert!(terminal.order_signing_context().is_none());

    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.account_data = Some(empty_account_data());
    terminal.account_data_address = Some(OTHER_ACCOUNT.to_string());
    assert!(terminal.connected_order_account_snapshot().is_none());
    terminal.account_data_address = Some(TEST_ACCOUNT.to_string());
    assert!(terminal.connected_order_account_snapshot().is_some());
}
