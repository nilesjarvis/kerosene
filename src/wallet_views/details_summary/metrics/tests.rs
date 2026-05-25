use super::*;

#[test]
fn wallet_margin_pct_rejects_invalid_or_ambiguous_inputs() {
    assert_eq!(wallet_margin_pct(Some(100.0), Some(25.0)), Some(25.0));
    assert_eq!(wallet_margin_pct(Some(0.0), Some(0.0)), Some(0.0));
    assert_eq!(wallet_margin_pct(Some(0.0), Some(1.0)), None);
    assert_eq!(wallet_margin_pct(None, Some(1.0)), None);
    assert_eq!(wallet_margin_pct(Some(100.0), None), None);
}
