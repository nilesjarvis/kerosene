use super::*;

#[test]
fn context_summary_shows_used_available_and_percentage() {
    assert_eq!(
        context_usage_summary(Some(12_500), Some(200_000)),
        "Context · 12.5K / 200.0K (6.2%)"
    );
    assert_eq!(
        context_usage_summary(None, Some(1_000_000)),
        "Context · — / 1.0M"
    );
    assert_eq!(context_usage_summary(None, None), "Context · — / —");
}

#[test]
fn compact_token_counts_keep_footer_readable() {
    assert_eq!(compact_token_count(999), "999");
    assert_eq!(compact_token_count(1_250), "1.2K");
    assert_eq!(compact_token_count(2_000_000), "2.0M");
}

#[test]
fn loading_activity_formats_short_and_long_elapsed_times() {
    assert_eq!(format_activity_elapsed(3_456), "3.5s");
    assert_eq!(format_activity_elapsed(62_340), "1m 2.3s");
}

#[test]
fn loading_drive_wave_keeps_pixels_dim_between_fronts() {
    assert_eq!(loading_pixel_alpha(0.8, 0.0), 0.15);
    assert!(loading_pixel_alpha(0.3, 0.0) > 0.95);
    assert!((0.15..=1.0).contains(&loading_pixel_alpha(0.12, 0.3)));
}
