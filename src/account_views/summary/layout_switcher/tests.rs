use super::*;

#[test]
fn layout_switcher_label_falls_back_for_missing_or_empty_names() {
    assert_eq!(layout_switcher_label(None, 14), "Layouts");
    assert_eq!(layout_switcher_label(Some("   "), 14), "Layouts");
}

#[test]
fn layout_switcher_label_truncates_long_names() {
    assert_eq!(
        layout_switcher_label(Some("Very Long Trading Layout"), 14),
        "Very Long T..."
    );
}

#[test]
fn layout_switcher_button_label_identifies_the_dropdown() {
    assert_eq!(layout_switcher_button_label(None, 14), "Layout");
    assert_eq!(
        layout_switcher_button_label(Some("Scalp"), 14),
        "Layout: Scalp"
    );
}
