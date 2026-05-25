use super::*;

#[test]
fn summary_percent_string_rejects_invalid_margin_ratio() {
    assert_eq!(summary_percent_string(Some(0.125)), "12.50%");
    assert_eq!(summary_percent_string(None), "Invalid data");
}
