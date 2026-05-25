use super::*;

#[test]
fn toggled_quantity_formats_usd_and_coin_modes() {
    assert_eq!(
        toggled_order_quantity_text(2.5, true, 100.0, 4),
        Some("250.00".to_string())
    );
    assert_eq!(
        toggled_order_quantity_text(250.0, false, 100.0, 4),
        Some("2.5000".to_string())
    );
    assert_eq!(
        toggled_order_quantity_text(12_345.0, true, 100.0, 2),
        Some("1,234,500.00".to_string())
    );
    assert_eq!(
        toggled_order_quantity_text(1_234_500.0, false, 100.0, 2),
        Some("12,345.00".to_string())
    );
}

#[test]
fn toggled_quantity_rejects_invalid_reference_values() {
    assert_eq!(toggled_order_quantity_text(2.5, true, 0.0, 4), None);
    assert_eq!(toggled_order_quantity_text(2.5, true, f64::NAN, 4), None);
}
