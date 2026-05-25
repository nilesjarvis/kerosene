use super::{quick_order_quantity_for_percentage, toggled_quick_order_quantity_text};

#[test]
fn quick_order_percentage_quantity_formats_usd_and_coin() {
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 1_000.0, true, Some(100.0), 4),
        "250.00"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 1_000.0, false, Some(100.0), 4),
        "2.5000"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 5_000_000.0, false, Some(100.0), 2),
        "12,500.00"
    );
}

#[test]
fn quick_order_percentage_quantity_rejects_invalid_inputs() {
    assert_eq!(
        quick_order_quantity_for_percentage(f32::NAN, 1_000.0, true, Some(100.0), 4),
        "0"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 0.0, true, Some(100.0), 4),
        "0"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 1_000.0, false, None, 4),
        "0"
    );
}

#[test]
fn toggled_quick_order_quantity_converts_when_reference_price_is_available() {
    assert_eq!(
        toggled_quick_order_quantity_text("2.5", true, Some(100.0), 4),
        "250.00"
    );
    assert_eq!(
        toggled_quick_order_quantity_text("250", false, Some(100.0), 4),
        "2.5000"
    );
    assert_eq!(
        toggled_quick_order_quantity_text("1,234,500.00", false, Some(100.0), 2),
        "12,345.00"
    );
}
