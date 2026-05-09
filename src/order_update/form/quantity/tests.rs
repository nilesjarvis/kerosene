use super::{order_percentage_for_quantity, quantity_for_percentage, toggled_order_quantity_text};

#[test]
fn order_percentage_uses_usd_quantity_directly() {
    assert_eq!(
        order_percentage_for_quantity(250.0, true, Some(100.0), 1_000.0),
        25.0
    );
}

#[test]
fn order_percentage_converts_coin_quantity_to_notional() {
    assert_eq!(
        order_percentage_for_quantity(2.5, false, Some(100.0), 1_000.0),
        25.0
    );
}

#[test]
fn order_percentage_clamps_and_rejects_invalid_values() {
    assert_eq!(
        order_percentage_for_quantity(2_000.0, true, Some(100.0), 1_000.0),
        100.0
    );
    assert_eq!(
        order_percentage_for_quantity(f64::NAN, true, Some(100.0), 1_000.0),
        0.0
    );
    assert_eq!(
        order_percentage_for_quantity(2.5, false, None, 1_000.0),
        0.0
    );
    assert_eq!(
        order_percentage_for_quantity(2.5, false, Some(100.0), 0.0),
        0.0
    );
}

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
}

#[test]
fn toggled_quantity_rejects_invalid_reference_values() {
    assert_eq!(toggled_order_quantity_text(2.5, true, 0.0, 4), None);
    assert_eq!(toggled_order_quantity_text(2.5, true, f64::NAN, 4), None);
}

#[test]
fn percentage_quantity_formats_usd_and_coin_amounts() {
    assert_eq!(
        quantity_for_percentage(25.0, 1_000.0, true, Some(100.0), 4),
        "250.00"
    );
    assert_eq!(
        quantity_for_percentage(25.0, 1_000.0, false, Some(100.0), 4),
        "2.5000"
    );
}

#[test]
fn percentage_quantity_clamps_or_zeros_invalid_values() {
    assert_eq!(
        quantity_for_percentage(150.0, 1_000.0, true, Some(100.0), 4),
        "1000.00"
    );
    assert_eq!(
        quantity_for_percentage(f32::NAN, 1_000.0, true, Some(100.0), 4),
        "0"
    );
    assert_eq!(quantity_for_percentage(25.0, 1_000.0, false, None, 4), "0");
}
