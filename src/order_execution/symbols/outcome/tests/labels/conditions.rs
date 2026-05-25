use super::outcome_info;

#[test]
fn binary_outcome_label_describes_threshold_and_expiry() {
    assert_eq!(
        outcome_info().market_label(),
        "BTC is above 76,886 at 2026-05-20 06:00 UTC"
    );
}

#[test]
fn bucket_outcome_label_describes_price_range() {
    let mut info = outcome_info();
    info.question_class = Some("priceBucket".to_string());
    info.question_underlying = Some("BTC".to_string());
    info.question_expiry = Some("20260520-0600".to_string());
    info.question_price_thresholds = vec!["75348".to_string(), "78423".to_string()];

    info.bucket_index = Some(0);
    assert_eq!(
        info.market_label(),
        "BTC is below 75,348 at 2026-05-20 06:00 UTC"
    );

    info.bucket_index = Some(1);
    assert_eq!(
        info.market_label(),
        "BTC is at or above 75,348 and below 78,423 at 2026-05-20 06:00 UTC"
    );

    info.bucket_index = Some(2);
    assert_eq!(
        info.market_label(),
        "BTC is at or above 78,423 at 2026-05-20 06:00 UTC"
    );
}

#[test]
fn no_side_outcome_label_describes_payoff_condition() {
    let mut info = outcome_info();
    info.side_index = 1;
    info.side_name = "No".to_string();

    assert_eq!(
        info.display_label(),
        "NO: BTC is at or below 76,886 at 2026-05-20 06:00 UTC"
    );

    info.question_class = Some("priceBucket".to_string());
    info.question_underlying = Some("BTC".to_string());
    info.question_expiry = Some("20260520-0600".to_string());
    info.question_price_thresholds = vec!["75348".to_string(), "78423".to_string()];
    info.bucket_index = Some(1);

    assert_eq!(
        info.display_label(),
        "NO: BTC is below 75,348 or at or above 78,423 at 2026-05-20 06:00 UTC"
    );
}

#[test]
fn named_question_outcome_label_uses_named_outcome() {
    let mut info = outcome_info();
    info.question_id = Some(19);
    info.question_name = Some("May CPI year-over-year".to_string());
    info.outcome_name = "Below 4.3%".to_string();
    info.description = "This outcome resolves to Yes if CPI is below 4.3%.".to_string();
    info.class = None;
    info.underlying = None;
    info.target_price = None;
    info.expiry = None;

    assert_eq!(info.market_label(), "May CPI year-over-year");
    assert_eq!(info.display_label(), "YES: Below 4.3%");

    info.side_index = 1;
    info.side_name = "No".to_string();
    assert_eq!(info.display_label(), "NO: not Below 4.3%");
}

#[test]
fn price_bucket_question_still_uses_bucket_condition() {
    let mut info = outcome_info();
    info.question_id = Some(18);
    info.question_name = Some("Recurring".to_string());
    info.question_class = Some("priceBucket".to_string());
    info.question_underlying = Some("BTC".to_string());
    info.question_expiry = Some("20260520-0600".to_string());
    info.question_price_thresholds = vec!["75348".to_string(), "78423".to_string()];
    info.outcome_name = "Recurring Named Outcome".to_string();
    info.bucket_index = Some(1);

    assert_eq!(
        info.display_label(),
        "YES: BTC is at or above 75,348 and below 78,423 at 2026-05-20 06:00 UTC"
    );
}

#[test]
fn short_side_condition_label_omits_expiry_details() {
    let mut info = outcome_info();

    assert_eq!(info.side_condition_short_label(), "BTC is above 76,886");

    info.side_index = 1;
    info.side_name = "No".to_string();
    assert_eq!(
        info.side_condition_short_label(),
        "BTC is at or below 76,886"
    );

    info.question_class = Some("priceBucket".to_string());
    info.question_underlying = Some("BTC".to_string());
    info.question_expiry = Some("20260520-0600".to_string());
    info.question_price_thresholds = vec!["75348".to_string(), "78423".to_string()];
    info.bucket_index = Some(1);

    assert_eq!(
        info.side_condition_short_label(),
        "BTC is below 75,348 or at or above 78,423"
    );
}
