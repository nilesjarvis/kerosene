use super::*;

#[test]
fn finite_value_filters_nonfinite_values() {
    assert_eq!(finite_value(1.25), Some(1.25));
    assert_eq!(finite_value(-1.25), Some(-1.25));
    assert_eq!(finite_value(f64::NAN), None);
    assert_eq!(finite_value(f64::INFINITY), None);
}

#[test]
fn positive_finite_value_filters_invalid_zero_and_negative_values() {
    assert_eq!(positive_finite_value(1.25), Some(1.25));
    assert_eq!(positive_finite_value(0.0), None);
    assert_eq!(positive_finite_value(-1.0), None);
    assert_eq!(positive_finite_value(f64::NAN), None);
    assert_eq!(positive_finite_value(f64::INFINITY), None);
}

#[test]
fn finite_number_parser_preserves_plain_wire_number_rules() {
    assert_eq!(parse_finite_number(" 12.5 "), Some(12.5));
    assert_eq!(parse_finite_number("-3"), Some(-3.0));
    assert_eq!(parse_finite_number("bad"), None);
    assert_eq!(parse_finite_number("NaN"), None);
    assert_eq!(parse_finite_number("inf"), None);
    assert_eq!(parse_finite_number("1,234"), None);
}

#[test]
fn finite_json_number_parser_accepts_plain_string_or_json_numbers() {
    assert_eq!(
        parse_finite_json_number(&serde_json::json!(" 12.5 ")),
        Some(12.5)
    );
    assert_eq!(
        parse_finite_json_number(&serde_json::json!(14.25)),
        Some(14.25)
    );
    assert_eq!(parse_finite_json_number(&serde_json::json!("NaN")), None);
    assert_eq!(parse_finite_json_number(&serde_json::json!("1,234")), None);
    assert_eq!(parse_finite_json_number(&serde_json::json!(null)), None);
}

#[test]
fn positive_finite_number_parser_rejects_nonpositive_values() {
    assert_eq!(parse_positive_finite_number(" 12.5 "), Some(12.5));
    assert_eq!(parse_positive_finite_number("0"), None);
    assert_eq!(parse_positive_finite_number("-3"), None);
    assert_eq!(parse_positive_finite_number("NaN"), None);
    assert_eq!(parse_positive_finite_number("1,234"), None);
}
