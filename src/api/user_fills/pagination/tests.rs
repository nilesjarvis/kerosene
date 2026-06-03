use super::next_user_fills_request;

#[test]
fn next_request_stops_when_page_is_not_full() {
    let (next, warning) = next_user_fills_request(100, 1_000, 1_999, 300);
    assert!(next.is_none());
    assert!(warning.is_none());
}

#[test]
fn next_request_advances_from_newest_time_for_full_page() {
    let (next, warning) = next_user_fills_request(100, 1_000, 2_000, 300);
    let next = next.expect("next request");
    assert_eq!(next.start_time, 300);
    assert_eq!(next.end_time, Some(1_000));
    assert!(warning.is_none());
}

#[test]
fn next_request_reuses_fixed_end_time_watermark() {
    let (next, warning) = next_user_fills_request(100, 9_999, 2_000, 300);
    let next = next.expect("next request");
    assert_eq!(next.start_time, 300);
    assert_eq!(next.end_time, Some(9_999));
    assert!(warning.is_none());
}

#[test]
fn next_request_stops_at_or_after_end_time() {
    let (next, warning) = next_user_fills_request(100, 300, 2_000, 300);
    assert!(next.is_none());
    assert!(warning.is_none());
}

#[test]
fn next_request_steps_forward_with_warning_when_timestamp_does_not_progress() {
    let (next, warning) = next_user_fills_request(100, 1_000, 2_000, 100);
    let next = next.expect("next request");
    assert_eq!(next.start_time, 101);
    assert_eq!(next.end_time, Some(1_000));
    assert!(
        warning
            .as_deref()
            .is_some_and(|message| message.contains("without timestamp progress"))
    );
}
