use super::normalized_layout_name;

#[test]
fn normalized_layout_name_trims_and_rejects_empty_names() {
    assert_eq!(
        normalized_layout_name("  Trading  "),
        Some("Trading".to_string())
    );
    assert_eq!(normalized_layout_name("   "), None);
}
