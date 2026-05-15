use super::*;

fn spot_balance(total: &str) -> SpotBalance {
    SpotBalance {
        coin: "PURR".to_string(),
        token: None,
        total: total.to_string(),
        hold: "0".to_string(),
        entry_ntl: "0".to_string(),
        supplied: None,
    }
}

#[test]
fn balance_number_parser_rejects_invalid_or_nonfinite_values() {
    assert_eq!(parse_balance_number(" 1.5 "), Some(1.5));
    assert_eq!(parse_balance_number("-2"), Some(-2.0));

    assert_eq!(parse_balance_number("bad"), None);
    assert_eq!(parse_balance_number("NaN"), None);
    assert_eq!(parse_balance_number("inf"), None);
}

#[test]
fn balance_visibility_keeps_invalid_totals_visible() {
    assert!(balance_has_visible_total(&spot_balance("bad")));
    assert!(balance_has_visible_total(&spot_balance("1")));
    assert!(!balance_has_visible_total(&spot_balance("0")));
}

#[test]
fn balance_amounts_mark_invalid_source_values() {
    assert_eq!(
        balance_amounts("USDC", Some(10.0), Some(7.0), Some(3.0)),
        (
            "$10.00".to_string(),
            "$7.00".to_string(),
            "$3.00".to_string()
        )
    );
    assert_eq!(
        balance_amounts("PURR", Some(10.0), None, Some(3.0)),
        (
            "10.000000".to_string(),
            "Invalid data".to_string(),
            "3.000000".to_string()
        )
    );
}

#[test]
fn entry_notional_marks_invalid_values() {
    assert_eq!(entry_notional_text(Some(12.5)), "$12.50");
    assert_eq!(entry_notional_text(Some(0.0)), "\u{2014}");
    assert_eq!(entry_notional_text(None), "Invalid data");
}
