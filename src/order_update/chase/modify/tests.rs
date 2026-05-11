use super::{chase_rate_limit_error, chase_terminal_modify_error};

#[test]
fn terminal_modify_error_detects_cancelled_or_filled_order_rejections() {
    assert!(chase_terminal_modify_error(
        "Error: Cannot modify canceled or filled order"
    ));
    assert!(chase_terminal_modify_error("cannot modify cancelled order"));
    assert!(chase_terminal_modify_error("cannot modify cancled order"));
    assert!(chase_terminal_modify_error("cannot modify filled order"));
}

#[test]
fn terminal_modify_error_rejects_unrelated_modify_errors() {
    assert!(!chase_terminal_modify_error(
        "Error: price too far from mark"
    ));
    assert!(!chase_terminal_modify_error("Cannot cancel filled order"));
    assert!(!chase_terminal_modify_error("Cannot modify order"));
}

#[test]
fn rate_limit_error_detects_exchange_throttle_messages() {
    assert!(chase_rate_limit_error("Error: rate limit exceeded"));
    assert!(chase_rate_limit_error("Too many requests"));
    assert!(chase_rate_limit_error("HTTP 429"));
    assert!(!chase_rate_limit_error("Cannot modify filled order"));
}
