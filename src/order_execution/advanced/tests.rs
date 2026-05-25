use super::AdvancedOrderKind;
use crate::app_state::TradingTerminal;
use crate::order_execution::PendingOrderAction;

#[test]
fn advanced_order_start_context_captures_account_and_trimmed_agent_key() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc".to_string());
    terminal.wallet_key_input = "  test-agent-key  ".to_string().into();

    let context = terminal
        .advanced_order_start_context(AdvancedOrderKind::Twap)
        .expect("valid advanced order context");

    assert_eq!(context.account_address, "0xabc");
    assert_eq!(&*context.agent_key, "test-agent-key");
}

#[test]
fn advanced_order_start_context_rejects_missing_agent_key() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc".to_string());
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
fn advanced_order_start_context_waits_for_pending_order_action() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc".to_string());
    terminal.wallet_key_input = "test-agent-key".to_string().into();
    terminal.pending_order_action = Some(PendingOrderAction::Buy);

    assert!(
        terminal
            .advanced_order_start_context(AdvancedOrderKind::Chase)
            .is_none()
    );
    assert_eq!(
        terminal.order_status.as_ref(),
        Some(&(
            "Wait for the pending order action to finish".to_string(),
            true
        ))
    );
}
