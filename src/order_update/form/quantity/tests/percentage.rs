use super::*;

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
fn percentage_quantity_formats_usd_and_coin_amounts() {
    assert_eq!(
        quantity_for_percentage(25.0, 1_000.0, true, Some(100.0), 4),
        "250.00"
    );
    assert_eq!(
        quantity_for_percentage(25.0, 1_000.0, false, Some(100.0), 4),
        "2.5000"
    );
    assert_eq!(
        quantity_for_percentage(25.0, 1_000_000.0, true, Some(100.0), 4),
        "250,000.00"
    );
    assert_eq!(
        quantity_for_percentage(25.0, 5_000_000.0, false, Some(100.0), 2),
        "12,500.00"
    );
}

#[test]
fn percentage_quantity_clamps_or_zeros_invalid_values() {
    assert_eq!(
        quantity_for_percentage(150.0, 1_000.0, true, Some(100.0), 4),
        "1,000.00"
    );
    assert_eq!(
        quantity_for_percentage(f32::NAN, 1_000.0, true, Some(100.0), 4),
        "0"
    );
    assert_eq!(quantity_for_percentage(25.0, 1_000.0, false, None, 4), "0");
}
