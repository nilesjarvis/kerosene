use super::*;

#[test]
fn invalid_inputs_are_marked_invalid() {
    let ctx = eur_context(1.25);

    assert_eq!(ctx.format_value(f64::NAN, 2), "Invalid data");
    assert_eq!(ctx.format_price(f64::INFINITY), "Invalid data");
    assert_eq!(ctx.format_chart_price(f64::INFINITY), "Invalid data");
    assert_eq!(ctx.format_signed_compact_value(f64::NAN), "Invalid data");
    assert_eq!(
        super::super::formatting::format_compact(f64::NAN),
        "Invalid data"
    );
    assert_eq!(format_compact_usd(f64::NAN), "Invalid data");
}
