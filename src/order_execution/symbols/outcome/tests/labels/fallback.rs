use super::outcome_info;

#[test]
fn fallback_outcome_label_is_explicit() {
    let mut info = outcome_info();
    info.question_class = Some("priceBucket".to_string());
    info.question_underlying = Some("BTC".to_string());
    info.question_expiry = Some("20260520-0600".to_string());
    info.is_question_fallback = true;

    assert_eq!(
        info.market_label(),
        "fallback / other settlement at 2026-05-20 06:00 UTC"
    );
}
