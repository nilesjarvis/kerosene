use super::next_user_fills_request;

#[test]
fn next_request_stops_when_page_is_not_full() {
    assert!(next_user_fills_request(100, 1999, 200, 300).is_none());
}

#[test]
fn next_request_uses_oldest_time_for_full_page() {
    let next = next_user_fills_request(100, 2000, 200, 300).expect("next request");
    assert_eq!(next.start_time, 100);
    assert_eq!(next.end_time, Some(200));
}

#[test]
fn next_request_steps_back_when_page_has_identical_timestamps() {
    let next = next_user_fills_request(100, 2000, 200, 200).expect("next request");
    assert_eq!(next.end_time, Some(199));
}

#[test]
fn next_request_stops_at_or_before_start_time() {
    assert!(next_user_fills_request(100, 2000, 100, 300).is_none());
    assert!(next_user_fills_request(100, 2000, 0, 0).is_none());
}
