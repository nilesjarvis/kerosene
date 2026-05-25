use super::*;

#[test]
fn invalid_data_placeholder_uses_shared_text() {
    assert_eq!(invalid_data_placeholder(), "Invalid data");
}

#[test]
fn not_available_placeholder_uses_shared_text() {
    assert_eq!(not_available_placeholder(), "n/a");
}
