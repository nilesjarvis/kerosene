use super::fixtures::context_or_panic;
use crate::order_execution::{MoveOrderContextError, PendingMoveOrderContext};
use zeroize::Zeroizing;

#[test]
fn pending_move_context_reuses_captured_agent_key_for_same_account() {
    let context = context_or_panic(PendingMoveOrderContext::new(
        "0xabc0000000000000000000000000000000000000",
        Zeroizing::new("original-agent-key".to_string()),
    ));

    assert_eq!(
        context.replacement_agent_key(Some("0xabc0000000000000000000000000000000000000")),
        Ok("original-agent-key".to_string().into())
    );
    assert_eq!(
        context.replacement_agent_key(Some(" 0xabc0000000000000000000000000000000000000 ")),
        Ok("original-agent-key".to_string().into())
    );
}

#[test]
fn pending_move_context_rejects_replacement_after_account_change() {
    let context = context_or_panic(PendingMoveOrderContext::new(
        "0xabc0000000000000000000000000000000000000",
        Zeroizing::new("original-agent-key".to_string()),
    ));

    assert_eq!(
        context.replacement_agent_key(Some("0xdef0000000000000000000000000000000000000")),
        Err(MoveOrderContextError::AccountChanged)
    );
    assert_eq!(
        context.replacement_agent_key(None),
        Err(MoveOrderContextError::AccountChanged)
    );
    assert_eq!(
        context.replacement_agent_key(Some("   ")),
        Err(MoveOrderContextError::AccountChanged)
    );
}

#[test]
fn pending_move_context_rejects_empty_agent_key() {
    assert!(matches!(
        PendingMoveOrderContext::new(
            "0xabc0000000000000000000000000000000000000",
            Zeroizing::new("   ".to_string()),
        ),
        Err(MoveOrderContextError::MissingAgentKey)
    ));
}
