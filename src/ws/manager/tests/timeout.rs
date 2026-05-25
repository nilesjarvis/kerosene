use super::*;

#[test]
fn read_loop_timeout_uses_stale_window_when_no_coalesced_frame() {
    let stale_in = Duration::from_secs(45);

    assert_eq!(read_loop_timeout(stale_in, None), stale_in);
}

#[test]
fn read_loop_timeout_uses_earliest_pending_deadline() {
    let stale_in = Duration::from_secs(45);
    let coalesce_due = Duration::from_millis(16);

    assert_eq!(
        read_loop_timeout(stale_in, Some(coalesce_due)),
        coalesce_due
    );
}

#[test]
fn read_loop_timeout_never_exceeds_stale_window() {
    let stale_in = Duration::from_secs(1);
    let coalesce_due = Duration::from_secs(24 * 3600);

    assert_eq!(read_loop_timeout(stale_in, Some(coalesce_due)), stale_in);
}
