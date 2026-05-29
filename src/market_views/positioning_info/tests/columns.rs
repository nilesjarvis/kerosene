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
    let columns = PositioningInfoColumns::for_width(450.0);

    assert!(columns.show_entry);
    assert!(!columns.show_size);
    assert!(!columns.show_liq);
    assert!(!columns.show_funding);
    assert!(!columns.show_account);
    assert!(columns.compact_money);
}

#[test]
fn positioning_change_columns_reserve_scrollbar_width() {
    let width = 900.0;
    let columns = PositioningChangeColumns::for_width(width);
    let content_width = PositioningInfoColumns::available_content_width(width);

    assert!((columns.total_width() - content_width).abs() < 0.01);
    assert!(columns.trader_width > POSITIONING_CHANGE_TRADER_MIN_WIDTH);
}

#[test]
fn positioning_change_trader_column_shows_compact_actions_before_positions_threshold() {
    let columns = PositioningChangeColumns::for_width(610.0);

    assert!(columns.trader_width < POSITIONING_TRADER_COMPACT_ACTIONS_MIN_WIDTH);
    assert_eq!(
        positioning_trader_action_visibility(
            columns.trader_width,
            POSITIONING_CHANGE_TRADER_COMPACT_ACTIONS_MIN_WIDTH,
        ),
        (true, false)
    );
}
