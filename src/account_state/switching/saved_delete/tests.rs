use crate::app_state::TradingTerminal;

#[test]
fn adjust_active_index_shifts_down_when_earlier_account_removed() {
    assert_eq!(TradingTerminal::adjust_active_index_after_removal(3, 1), 2);
}

#[test]
fn adjust_active_index_keeps_value_when_later_account_removed() {
    assert_eq!(TradingTerminal::adjust_active_index_after_removal(2, 5), 2);
}

#[test]
fn adjust_active_index_keeps_value_when_active_itself_is_removed() {
    // The active account being removed is handled separately by the
    // fallback-switch path; the index adjustment alone should not shift.
    assert_eq!(TradingTerminal::adjust_active_index_after_removal(4, 4), 4);
}

#[test]
fn adjust_active_index_handles_zero_indexes() {
    assert_eq!(TradingTerminal::adjust_active_index_after_removal(0, 0), 0);
    assert_eq!(TradingTerminal::adjust_active_index_after_removal(1, 0), 0);
}
