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

#[test]
fn user_fill_accepts_optional_stable_identity_metadata() {
    let fill = user_fill_or_panic(user_fill_value_with_identity(123, "0xabc"));

    assert_eq!(fill.tid, Some(123));
    assert_eq!(fill.hash.as_deref(), Some("0xabc"));
}

#[test]
fn user_fill_hash_dedup_key_includes_fill_fields_without_tid() {
    let mut first = user_fill_or_panic(user_fill_value_with_identity(123, "0xabc"));
    let mut second = first.clone();
    first.tid = None;
    second.tid = None;
    second.px = "101".to_string();

    assert_ne!(first.dedup_key(), second.dedup_key());
}
