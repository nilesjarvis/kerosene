use super::*;

#[test]
fn positioning_columns_expand_to_span_wide_panes() {
    let width = 1_200.0;
    let columns = PositioningInfoColumns::for_width(width);
    let content_width = PositioningInfoColumns::available_content_width(width);

    assert!((columns.total_width() - content_width).abs() < 0.01);
    assert!(columns.trader_width > POSITIONING_TRADER_MIN_WIDTH);
    assert!(columns.size_width > POSITIONING_SIZE_WIDTH);
    assert!(columns.show_size);
    assert!(columns.show_entry);
    assert!(columns.show_liq);
    assert!(columns.show_funding);
    assert!(columns.show_account);
    assert!(!columns.compact_money);
}

#[test]
fn positioning_columns_shrink_trader_width_on_narrow_panes() {
    let width = 300.0;
    let columns = PositioningInfoColumns::for_width(width);
    let content_width = PositioningInfoColumns::available_content_width(width);

    assert!((columns.total_width() - content_width).abs() < 0.01);
    assert!(columns.trader_width < POSITIONING_TRADER_MIN_WIDTH);
    assert!(!columns.show_size);
    assert!(!columns.show_entry);
    assert!(!columns.show_liq);
    assert!(!columns.show_funding);
    assert!(!columns.show_account);
    assert!(columns.compact_money);
}

#[test]
fn positioning_columns_hide_requested_columns_in_compact_panes() {
    // At this width only the base columns fit comfortably; the reveal margin
    // keeps optional columns hidden until they have breathing room.
    let columns = PositioningInfoColumns::for_width(450.0);

    assert!(!columns.show_entry);
    assert!(!columns.show_size);
    assert!(!columns.show_liq);
    assert!(!columns.show_funding);
    assert!(!columns.show_account);
    assert!(columns.compact_money);
}

#[test]
fn positioning_columns_reveal_optional_columns_progressively() {
    // The first optional column (Entry) appears at a mid width.
    let mid = PositioningInfoColumns::for_width(620.0);
    assert!(mid.show_entry);
    assert!(!mid.show_account);
    assert!(mid.compact_money);

    // A wider pane reveals strictly more optional columns than a narrower one.
    let wider = PositioningInfoColumns::for_width(900.0);
    let mid_optional = usize::from(mid.show_size)
        + usize::from(mid.show_entry)
        + usize::from(mid.show_liq)
        + usize::from(mid.show_funding)
        + usize::from(mid.show_account);
    let wider_optional = usize::from(wider.show_size)
        + usize::from(wider.show_entry)
        + usize::from(wider.show_liq)
        + usize::from(wider.show_funding)
        + usize::from(wider.show_account);
    assert!(wider_optional > mid_optional);
}

#[test]
fn positioning_trader_actions_match_change_tab_label_width() {
    // The change tab uses a 120px address slot and still swaps the address for
    // the three-button action pill on hover; positions should match that.
    assert!(!positioning_trader_actions_enabled(119.0));
    assert!(positioning_trader_actions_enabled(120.0));
}
