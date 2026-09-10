use super::fixtures::context_or_panic;
use crate::order_execution::{MoveOrderContextError, PendingMoveOrderContext};
use zeroize::Zeroizing;

#[test]
fn pending_move_context_reuses_captured_agent_key_for_same_account() {
    let context = context_or_panic(PendingMoveOrderContext::new(
        "0xabc0000000000000000000000000000000000000",
        Zeroizing::new("original-agent-key".to_string()).into(),
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
        Zeroizing::new("original-agent-key".to_string()).into(),
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
    let mut key: crate::signing::CapturedAgentKey = "cleared-key".into();
    key.clear();
    assert!(matches!(
        PendingMoveOrderContext::new("0xabc0000000000000000000000000000000000000", key,),
        Err(MoveOrderContextError::MissingAgentKey)
    ));
}

#[test]
fn pending_move_context_preserves_subaccount_for_replacement() {
    let child = "0xabc0000000000000000000000000000000000000";
    let parent = "0xdef0000000000000000000000000000000000000";
    let key = crate::signing::CapturedAgentKey::for_account(
        Zeroizing::new("parent-agent-key".to_string()),
        Some(child),
    )
    .expect("subaccount key");
    let context = context_or_panic(PendingMoveOrderContext::new(child, key));

    let replacement = context
        .replacement_agent_key(Some(child))
        .expect("same account");
    assert_eq!(replacement.vault_address(), Some(child));
    assert_eq!(replacement.as_str(), "parent-agent-key");
    assert_eq!(
        context.replacement_agent_key(Some(parent)),
        Err(MoveOrderContextError::AccountChanged)
    );
}
