use super::*;

#[test]
fn outcome_order_preparation_builds_spot_like_probability_payload() {
    let terminal = terminal_for_outcome_order(outcome_symbol("#650", 100_000_650, false));
    let sym = first_symbol_or_panic(&terminal);

    let prepared = prepared_order_or_panic(&terminal, sym, true);

    assert_eq!(
        prepared,
        PreparedOrderSubmission {
            asset: 100_000_650,
            is_buy: true,
            price: "0.42123".to_string(),
            size: "3".to_string(),
            order_kind: OrderKind::Limit,
            reduce_only: false,
            is_outcome: true,
        }
    );
}

#[test]
fn execute_order_rejects_non_tradable_fallback_outcome_before_signing() {
    let mut terminal = terminal_for_outcome_order(outcome_symbol("#660", 100_000_660, true));

    let _task = terminal.execute_order(true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("not a tradable market"));
    assert!(terminal.pending_order_action.is_none());
}
