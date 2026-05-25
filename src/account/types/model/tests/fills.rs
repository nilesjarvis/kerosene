use super::*;

#[test]
fn user_fill_preserves_optional_order_id_metadata() {
    let fill = user_fill_or_panic(user_fill_value_with_oid(Some(42)));

    assert_eq!(fill.oid, Some(42));
}

#[test]
fn user_fill_accepts_missing_order_id_metadata() {
    let fill = user_fill_or_panic(user_fill_value_with_oid(None));

    assert_eq!(fill.oid, None);
}
