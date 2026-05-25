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
