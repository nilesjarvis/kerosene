use super::super::twap_cancel_target_matches;

#[test]
fn twap_cancel_target_matches_by_oid_or_cloid() {
    assert!(twap_cancel_target_matches(
        Some(42),
        Some("0xabc"),
        Some(42),
        None
    ));
    assert!(twap_cancel_target_matches(
        None,
        Some("0xabc"),
        None,
        Some("0xabc")
    ));
    assert!(!twap_cancel_target_matches(
        Some(42),
        Some("0xabc"),
        Some(43),
        Some("0xdef")
    ));
}
