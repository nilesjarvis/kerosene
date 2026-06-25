use super::*;

#[test]
fn positioning_info_filters_track_side_and_sort_changes() {
    let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());

    assert!(!instance.has_active_filters());

    instance.side = PositioningInfoSide::Long;
    assert!(instance.has_active_filters());

    instance.reset_filters();
    assert!(!instance.has_active_filters());

    instance.sort_field = PositioningInfoSortField::NotionalSize;
    assert!(instance.has_active_filters());

    instance.reset_filters();
    instance.sort_direction = config::SortDirection::Ascending;
    assert!(instance.has_active_filters());

    instance.reset_filters();
    instance.entry_min_input = "20".to_string();
    assert!(instance.has_active_filters());
    instance.entry_max_input = "30".to_string();
    instance.reset_filters();
    assert!(!instance.has_active_filters());
    assert!(instance.entry_min_input.is_empty());
    assert!(instance.entry_max_input.is_empty());
}

#[test]
fn positioning_info_removed_copy_sort_normalizes_to_default_sort() {
    let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());
    instance.side = PositioningInfoSide::Short;
    instance.sort_field = PositioningInfoSortField::CopyScore;
    instance.sort_direction = config::SortDirection::Ascending;

    instance.normalize_removed_filters();

    assert_eq!(instance.side, PositioningInfoSide::Short);
    assert_eq!(instance.sort_field, PositioningInfoSortField::UnrealizedPnl);
    assert_eq!(instance.sort_direction, config::SortDirection::Descending);
}

#[test]
fn positioning_notional_and_size_sorts_use_hyperdash_notional_enum_name() {
    assert_eq!(
        PositioningInfoSortField::NotionalSize.api_field(),
        "notional"
    );
    assert_eq!(PositioningInfoSortField::Size.api_field(), "notional");
}

#[test]
fn positioning_entry_sort_uses_hyperdash_entry_price_field() {
    assert_eq!(
        PositioningInfoSortField::EntryPrice.api_field(),
        "entryPrice"
    );
}

#[test]
fn positioning_change_nav_label_uses_delta_symbol() {
    assert_eq!(PositioningInfoPage::Change.label(), "\u{0394} Change");
}

#[test]
fn positioning_change_defaults_to_short_timeframe_and_largest_change() {
    let instance = PositioningInfoInstance::new(7, "HYPE".to_string());

    assert_eq!(
        instance.change_timeframe,
        PositioningInfoChangeTimeframe::FifteenMinutes
    );
    assert_eq!(
        instance.change_sort_field,
        PositioningInfoChangeSortField::Change
    );
    assert_eq!(
        instance.change_sort_direction,
        config::SortDirection::Descending
    );
}
