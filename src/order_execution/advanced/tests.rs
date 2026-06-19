use super::{AdvancedOrderKind, AdvancedOrderStartSnapshot, TwapOrderStartSnapshot};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config::{AccountProfile, MarketUniverseConfig};
use crate::order_execution::{OneShotPlacementContext, OrderSurface, PendingOrderAction};
use crate::order_update::PendingOneShotStatusRequest;
use crate::signing::{ExchangeOrderKind, OrderKind};
use crate::twap_state::TwapOrderForm;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

fn connect_test_account(terminal: &mut TradingTerminal) {
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.wallet_address_input = TEST_ACCOUNT.to_string();
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: TEST_ACCOUNT.to_string(),
        agent_key: sensitive_string("").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }];
    terminal.active_account_index = 0;
}

fn pending_one_shot_status_request() -> PendingOneShotStatusRequest {
    PendingOneShotStatusRequest::new(
        7,
        &OneShotPlacementContext {
            account_address: TEST_ACCOUNT.to_string(),
            cloid: "0x00000000000000000000000000000004".to_string(),
            surface: OrderSurface::Ticket,
            symbol_key: "BTC".to_string(),
            order_kind: ExchangeOrderKind::Market,
        },
    )
}

fn advanced_snapshot() -> AdvancedOrderStartSnapshot {
    AdvancedOrderStartSnapshot {
        order_kind: OrderKind::Limit,
        symbol_key: "SECRETCOIN".into(),
        quantity_input: "quantity-secret".into(),
        quantity_is_usd: true,
        reduce_only: true,
        market_universe: MarketUniverseConfig::hip3_dex("secret-dex"),
    }
}

#[test]
fn advanced_order_start_snapshot_debug_redacts_symbol_and_quantity() {
    let debug = format!("{:?}", advanced_snapshot());

    assert!(debug.contains("AdvancedOrderStartSnapshot"));
    assert!(debug.contains("order_kind: Limit"));
    assert!(debug.contains("quantity_is_usd: true"));
    assert!(debug.contains("reduce_only: true"));
    assert!(!debug.contains("SECRETCOIN"));
    assert!(!debug.contains("quantity-secret"));
}

#[test]
fn twap_order_start_snapshot_debug_redacts_order_and_twap_form() {
    let snapshot = TwapOrderStartSnapshot {
        order: advanced_snapshot(),
        twap_form: TwapOrderForm {
            duration_minutes: "duration-secret".into(),
            slices: "slices-secret".into(),
            min_price: "min-price-secret".into(),
            max_price: "max-price-secret".into(),
            randomize: false,
        },
    };

    let debug = format!("{snapshot:?}");

    assert!(debug.contains("TwapOrderStartSnapshot"));
    assert!(debug.contains("order: AdvancedOrderStartSnapshot"));
    assert!(debug.contains("twap_form: \"<redacted>\""));
    assert!(!debug.contains("SECRETCOIN"));
    assert!(!debug.contains("quantity-secret"));
    assert!(!debug.contains("duration-secret"));
    assert!(!debug.contains("slices-secret"));
    assert!(!debug.contains("min-price-secret"));
    assert!(!debug.contains("max-price-secret"));
}

#[test]
fn advanced_order_start_context_captures_account_and_trimmed_agent_key() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("  test-agent-key  ");

    let context = terminal
        .advanced_order_start_context(AdvancedOrderKind::Twap)
        .expect("valid advanced order context");

    assert_eq!(context.account_address, TEST_ACCOUNT);
    assert_eq!(context.agent_key.as_str(), "test-agent-key");
}

#[test]
fn advanced_order_start_context_rejects_missing_agent_key() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.wallet_key_input.clear();

    assert!(
        terminal
            .advanced_order_start_context(AdvancedOrderKind::Chase)
            .is_none()
    );
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&("Connect wallet and enter agent key first".to_string(), true))
    );
}

#[test]
fn advanced_order_start_preflight_rejects_missing_agent_key() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);

    assert!(!terminal.advanced_order_start_preflight_ready(AdvancedOrderKind::Chase));
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&("Connect wallet and enter agent key first".to_string(), true))
    );
}

#[test]
fn advanced_order_start_context_rejects_blank_connected_address() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("   ".to_string());
    terminal.set_committed_agent_key_for_test("test-agent-key");

    assert!(
        terminal
            .advanced_order_start_context(AdvancedOrderKind::Chase)
            .is_none()
    );
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&("Connect wallet and enter agent key first".to_string(), true))
    );
}

#[test]
fn advanced_order_start_context_waits_for_pending_order_action() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("test-agent-key");
    terminal.pending_order_action = Some(PendingOrderAction::Buy);

    assert!(
        terminal
            .advanced_order_start_context(AdvancedOrderKind::Chase)
            .is_none()
    );
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&(
            "Wait for pending trading requests to finish before starting a chase".to_string(),
            true
        ))
    );
}

#[test]
fn advanced_order_start_context_waits_for_pending_one_shot_status() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("test-agent-key");
    terminal.pending_one_shot_status_request = Some(pending_one_shot_status_request());

    assert!(
        terminal
            .advanced_order_start_context(AdvancedOrderKind::Twap)
            .is_none()
    );
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&(
            "Wait for pending trading requests to finish before starting a TWAP".to_string(),
            true
        ))
    );
    assert!(terminal.pending_one_shot_status_request.is_some());
}

#[test]
fn advanced_order_start_context_rejects_pending_account_reconciliation() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("test-agent-key");
    terminal.account_reconciliation_required = true;

    assert!(
        terminal
            .advanced_order_start_context(AdvancedOrderKind::Twap)
            .is_none()
    );
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&(
            "Account refresh pending; wait for fresh account data before starting a TWAP"
                .to_string(),
            true
        ))
    );
}

#[test]
fn advanced_order_start_context_rejects_active_wallet_mismatch() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.wallet_address_input = "0xdef0000000000000000000000000000000000000".to_string();
    terminal.accounts[0].wallet_address = "0xdef0000000000000000000000000000000000000".to_string();
    terminal.set_committed_agent_key_for_test("test-agent-key");

    assert!(
        terminal
            .advanced_order_start_context(AdvancedOrderKind::Chase)
            .is_none()
    );
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
fn advanced_order_start_preflight_rejects_active_wallet_mismatch() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.wallet_address_input = "0xdef0000000000000000000000000000000000000".to_string();
    terminal.accounts[0].wallet_address = "0xdef0000000000000000000000000000000000000".to_string();
    terminal.set_committed_agent_key_for_test("test-agent-key");

    assert!(!terminal.advanced_order_start_preflight_ready(AdvancedOrderKind::Chase));
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&(
            "Connected wallet no longer matches the active account; reconnect before trading"
                .to_string(),
            true
        ))
    );
}
