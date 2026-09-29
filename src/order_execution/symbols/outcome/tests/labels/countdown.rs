use super::{outcome_info, utc_ms};

#[test]
fn countdown_label_uses_current_user_clock_distance() {
    let info = outcome_info();
    let now_ms = utc_ms(2026, 5, 19, 13, 45);

    assert_eq!(
        info.side_condition_label_with_countdown(now_ms),
        "BTC is above 76,886 at 2026-05-20 06:00 UTC (16h 15m left)"
    );
}

#[test]
fn countdown_label_marks_expired_markets() {
    let info = outcome_info();
    let now_ms = utc_ms(2026, 5, 20, 6, 1);

    assert_eq!(
        info.side_condition_label_with_countdown(now_ms),
        "BTC is above 76,886 at 2026-05-20 06:00 UTC (expired)"
    );
}

#[test]
fn expiry_suffix_is_consistent_across_binary_bucket_and_fallback_labels() {
    let now_ms = utc_ms(2026, 5, 20, 5, 0);
    let cases = [
        (false, false, "Yes", "BTC is above 76,886"),
        (false, false, "No", "BTC is at or below 76,886"),
        (true, false, "Yes", "BTC is at or above 10 and below 20"),
        (true, false, "No", "BTC is below 10 or at or above 20"),
        (true, true, "Yes", "fallback / other settlement"),
        (true, true, "No", "a named BTC bucket settles"),
    ];
    for (bucket, fallback, side, condition) in cases {
        for (expiry, suffix) in [
            (Some("20260520-0600"), " at 2026-05-20 06:00 UTC (1h left)"),
            (Some("20260520-0500"), " at 2026-05-20 05:00 UTC (expired)"),
            (Some("unavailable"), " at unavailable"),
            (None, ""),
        ] {
            let mut info = outcome_info();
            info.side_name = side.to_string();
            info.is_question_fallback = fallback;
            info.expiry = expiry.map(str::to_string);
            if bucket {
                info.question_class = Some("priceBucket".to_string());
                info.question_underlying = Some("BTC".to_string());
                info.question_price_thresholds = vec!["10".to_string(), "20".to_string()];
                info.bucket_index = Some(1);
                info.question_expiry = expiry.map(str::to_string);
            }
            assert_eq!(info.side_condition_short_label(), condition);
            assert_eq!(
                info.side_condition_label_with_countdown(now_ms),
                format!("{condition}{suffix}")
            );
        }
    }
}

#[test]
fn countdown_keeps_expiry_precedence_and_unrepresentable_clock_behavior() {
    let mut info = outcome_info();
    let now_ms = utc_ms(2026, 5, 20, 5, 0);
    assert_eq!(info.time_left_label(now_ms).as_deref(), Some("1h"));
    assert_eq!(info.time_left_label(u64::MAX), None);
    assert_eq!(
        info.side_condition_label_with_countdown(u64::MAX),
        info.side_condition_label()
    );

    info.question_expiry = Some("invalid".to_string());
    assert_eq!(info.time_left_label(now_ms), None);
    info.question_expiry = Some("20260520-0700".to_string());
    assert_eq!(info.time_left_label(now_ms).as_deref(), Some("2h"));
    // Non-bucket condition labels retain the outcome's own expiry.
    assert_eq!(
        info.side_condition_label_with_countdown(now_ms),
        "BTC is above 76,886 at 2026-05-20 06:00 UTC (1h left)"
    );

    info.question_expiry = None;
    info.expiry = Some("19690101-0000".to_string());
    assert_eq!(info.time_left_label(0).as_deref(), Some("expired"));
    assert_eq!(
        info.side_condition_label_with_countdown(0),
        "BTC is above 76,886 at 1969-01-01 00:00 UTC (expired)"
    );

    info.expiry = Some("20260520-0600".to_string());
    info.class = None;
    assert_eq!(
        info.market_label_with_countdown(now_ms),
        "BTC is above 76,886 at 2026-05-20 06:00 UTC (1h left)"
    );
}
