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

#[test]
fn master_and_subaccount_signatures_match_official_python_sdk_vectors() {
    // Public test fixtures from hyperliquid-python-sdk/tests/signing_test.py:
    // test_l1_action_signing_matches and test_l1_action_signing_matches_with_vault.
    // This key is a published synthetic fixture, never an account credential.
    // https://github.com/hyperliquid-dex/hyperliquid-python-sdk/blob/master/tests/signing_test.py
    const SDK_TEST_KEY: &str = "0x0123456789012345678901234567890123456789012345678901234567890123";

    #[derive(serde::Serialize)]
    struct DummyAction {
        #[serde(rename = "type")]
        action_type: &'static str,
        num: u64,
    }

    let packed = rmp_serde::to_vec_named(&DummyAction {
        action_type: "dummy",
        num: 100_000_000_000,
    })
    .expect("SDK dummy action should encode");
    for (target, expected_r, expected_s, expected_v) in [
        (
            None,
            "53749d5b30552aeb2fca34b530185976545bb22d0b3ce6f62e31be961a59298",
            "755c40ba9bf05223521753995abb2f73ab3229be8ec921f350cb447e384d8ed8",
            27,
        ),
        (
            Some("0x1719884eb866cb12b2287399b15f7db5e7d775ea"),
            "3c548db75e479f8012acf3000ca3a6b05606bc2ec0c29c50c515066a326239",
            "4d402be7396ce74fbba3795769cda45aec00dc3125a984f2a9f23177b190da2c",
            28,
        ),
    ] {
        let signature = sign_l1_action(SDK_TEST_KEY, &packed, target, 0, None)
            .expect("SDK fixture should sign");

        // Python serializes integers without leading zeroes; Rust emits bytes.
        assert_eq!(signature["r"], format!("0x{expected_r:0>64}"));
        assert_eq!(signature["s"], format!("0x{expected_s:0>64}"));
        assert_eq!(signature["v"], expected_v);
    }
}
