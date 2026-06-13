use crate::signing::crypto::{action_hash_bytes, sign_l1_action};

const TEST_PRIVATE_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000001";

fn action_hash_or_panic(expires_after: Option<u64>) -> [u8; 32] {
    match action_hash_bytes(b"{}", None, 1, expires_after) {
        Ok(hash) => hash,
        Err(error) => panic!("hash should be generated: {error}"),
    }
}

#[test]
fn action_hash_rejects_invalid_vault_hex() {
    let result = action_hash_bytes(b"{}", Some("0xnot-hex"), 1, None);

    assert!(result.is_err());
}

#[test]
fn action_hash_rejects_invalid_vault_length() {
    let result = action_hash_bytes(b"{}", Some("0x1234"), 1, None);

    assert!(result.is_err());
}

#[test]
fn action_hash_accepts_valid_vault_address() {
    let result = action_hash_bytes(
        b"{}",
        Some("0x0000000000000000000000000000000000000000"),
        1,
        None,
    );

    assert!(result.is_ok());
}

#[test]
fn action_hash_changes_when_expires_after_is_included() {
    let without_expiry = action_hash_or_panic(None);
    let with_expiry = action_hash_or_panic(Some(1_700_000_000_000));

    assert_ne!(without_expiry, with_expiry);
}

#[test]
fn sign_l1_action_accepts_prefixed_and_unprefixed_private_keys() {
    let unprefixed =
        sign_l1_action(TEST_PRIVATE_KEY, b"{}", None, 1, None).expect("unprefixed key should sign");
    let prefixed = sign_l1_action(&format!("0x{TEST_PRIVATE_KEY}"), b"{}", None, 1, None)
        .expect("prefixed key should sign");

    assert_eq!(unprefixed, prefixed);
}

#[test]
fn sign_l1_action_rejects_invalid_key_without_echoing_input() {
    let invalid_key = format!("{TEST_PRIVATE_KEY}ff");
    let error =
        sign_l1_action(&invalid_key, b"{}", None, 1, None).expect_err("invalid length should fail");

    assert!(error.contains("Invalid private key hex"));
    assert!(!error.contains(&invalid_key));
    assert!(!error.contains(TEST_PRIVATE_KEY));
}
