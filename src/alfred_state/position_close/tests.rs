use super::*;

#[test]
fn parses_full_position_close() {
    let intent = parse_close_position_intent("close HYPE").expect("close intent");

    assert_eq!(intent.symbol.as_deref(), Some("HYPE"));
    assert_eq!(intent.fraction, None);
    assert_eq!(intent.error, None);
}

#[test]
fn parses_fractional_position_close() {
    let intent = parse_close_position_intent("close hype 25").expect("close intent");

    assert_eq!(intent.symbol.as_deref(), Some("hype"));
    assert_eq!(intent.fraction, Some(0.25));
    assert_eq!(intent.error, None);
}

#[test]
fn parses_percent_sign_position_close() {
    let intent = parse_close_position_intent("close hype 12.5%").expect("close intent");

    assert_eq!(intent.fraction, Some(0.125));
    assert_eq!(intent.error, None);
}

#[test]
fn parses_fraction_before_ticker_position_close() {
    let intent = parse_close_position_intent("close 100 hype").expect("close intent");

    assert_eq!(intent.symbol.as_deref(), Some("hype"));
    assert_eq!(intent.fraction, Some(1.0));
    assert_eq!(intent.error, None);
}

#[test]
fn parses_percent_sign_before_ticker_position_close() {
    let intent = parse_close_position_intent("close 100% HYPE").expect("close intent");

    assert_eq!(intent.symbol.as_deref(), Some("HYPE"));
    assert_eq!(intent.fraction, Some(1.0));
    assert_eq!(intent.error, None);
}

#[test]
fn rejects_invalid_close_percentages() {
    let intent = parse_close_position_intent("close hype 125").expect("close intent");

    assert_eq!(
        intent.error.as_deref(),
        Some("Use a close percentage from 1 to 100")
    );
}

#[test]
fn ignores_non_close_queries() {
    assert_eq!(parse_close_position_intent("buy HYPE"), None);
    assert_eq!(parse_close_position_intent("nuke"), None);
}
