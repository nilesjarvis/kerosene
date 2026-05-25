use super::*;

#[test]
fn wallet_position_size_marks_invalid_values() {
    assert_eq!(format_wallet_position_size(Some(-2.5)), "2.5000");
    assert_eq!(format_wallet_position_size(None), "Invalid data");
}
