use super::*;

#[test]
fn position_price_overlays_hide_when_obscured() {
    assert!(!should_draw_position_price_overlays(true));
    assert!(should_draw_position_price_overlays(false));
}

#[test]
fn position_price_labels_redact_when_obscured() {
    assert_eq!(position_entry_badge_label(12345.67, true), "ENTRY");
    assert_eq!(position_liquidation_badge_label(9800.0, true), "LIQ");
}

#[test]
fn position_price_labels_show_prices_when_not_obscured() {
    assert_eq!(position_entry_badge_label(12345.67, false), "12,345.7");
    assert_eq!(
        position_liquidation_badge_label(9800.0, false),
        "Liq 9,800.0"
    );
}
