use super::feedback::{chart_header_changed_text, format_signed_usd_change};

mod funding;

#[test]
fn changed_text_highlights_only_changed_decimal_digit() {
    let parts = chart_header_changed_text("82,543.2", "82,543.3").expect("changed text");

    assert_eq!(parts.before, "82,543.");
    assert_eq!(parts.changed, "3");
    assert_eq!(parts.after, "");
}

#[test]
fn changed_text_keeps_shared_suffix_when_middle_digits_change() {
    let parts = chart_header_changed_text("82,543.2", "82,613.2").expect("changed text");

    assert_eq!(parts.before, "82,");
    assert_eq!(parts.changed, "61");
    assert_eq!(parts.after, "3.2");
}

#[test]
fn changed_text_ignores_equal_formatted_prices() {
    assert_eq!(chart_header_changed_text("82,543.2", "82,543.2"), None);
}

#[test]
fn signed_usd_change_marks_nonfinite_values_invalid() {
    assert_eq!(format_signed_usd_change(12.5), "+$12.50");
    assert_eq!(format_signed_usd_change(-12.5), "-$12.50");
    assert_eq!(format_signed_usd_change(f64::NAN), "Invalid data");
}
