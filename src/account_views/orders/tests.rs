use super::*;

#[test]
fn open_order_positive_parser_rejects_invalid_zero_or_nonfinite_values() {
    assert_eq!(parse_open_order_positive(" 1.25 "), Some(1.25));
    assert_eq!(parse_open_order_positive("0"), None);
    assert_eq!(parse_open_order_positive("-1"), None);
    assert_eq!(parse_open_order_positive("bad"), None);
    assert_eq!(parse_open_order_positive("NaN"), None);
    assert_eq!(parse_open_order_positive("inf"), None);
}

#[test]
fn chase_inputs_require_known_side_size_and_price() {
    assert_eq!(
        open_order_chase_inputs("B", Some(2.0), Some(100.0)),
        Some((true, 2.0, 100.0))
    );
    assert_eq!(
        open_order_chase_inputs("A", Some(2.0), Some(100.0)),
        Some((false, 2.0, 100.0))
    );
    assert_eq!(open_order_chase_inputs("bad", Some(2.0), Some(100.0)), None);
    assert_eq!(open_order_chase_inputs("B", None, Some(100.0)), None);
    assert_eq!(open_order_chase_inputs("B", Some(2.0), None), None);
}

#[test]
fn open_order_formatters_mark_invalid_values() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(
        format_open_order_price(Some(100.0), false, &denomination),
        "$100.00"
    );
    assert_eq!(
        format_open_order_price(Some(0.42), true, &denomination),
        "0.4200"
    );
    assert_eq!(
        format_open_order_price(None, false, &denomination),
        "Invalid data"
    );
    assert_eq!(format_open_order_size(Some(2.0), "2.0000"), "2.0000");
    assert_eq!(format_open_order_size(None, "bad"), "Invalid data");
}
