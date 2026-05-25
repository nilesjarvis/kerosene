use super::*;

#[test]
fn invalid_account_data_uses_shared_placeholder_text() {
    assert_eq!(invalid_account_data(), "Invalid data");
}
