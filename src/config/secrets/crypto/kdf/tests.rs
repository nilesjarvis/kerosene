use crate::config::secrets::model::SECRET_SALT_LEN;

use super::*;

fn test_kdf_config() -> SecretKdfConfig {
    let mut kdf = default_secret_kdf_config(&[7_u8; SECRET_SALT_LEN]);
    kdf.memory_kib = 64;
    kdf.iterations = 1;
    kdf.lanes = 1;
    kdf
}

#[test]
fn default_kdf_config_encodes_salt_and_default_params() {
    let kdf = default_secret_kdf_config(&[1_u8; SECRET_SALT_LEN]);

    assert_eq!(kdf.algorithm, "argon2id");
    assert_eq!(
        decode_secret_field("salt", &kdf.salt).expect("salt"),
        [1_u8; SECRET_SALT_LEN]
    );
    assert_eq!(kdf.memory_kib, DEFAULT_ARGON2_MEMORY_KIB);
    assert_eq!(kdf.iterations, DEFAULT_ARGON2_ITERATIONS);
    assert_eq!(kdf.lanes, DEFAULT_ARGON2_LANES);
}

#[test]
fn derive_secret_key_rejects_empty_password() {
    let error = derive_secret_key("", &test_kdf_config()).expect_err("empty password");

    assert_eq!(error, "credential password is required");
}

#[test]
fn derive_secret_key_rejects_unsupported_algorithm() {
    let mut kdf = test_kdf_config();
    kdf.algorithm = "scrypt-sensitive-marker".to_string();

    let error = derive_secret_key("password", &kdf).expect_err("unsupported algorithm");

    assert_eq!(error, "unsupported secret KDF");
    assert!(!error.contains("sensitive-marker"));
}

#[test]
fn derive_secret_key_rejects_invalid_salt_encoding() {
    let mut kdf = test_kdf_config();
    kdf.salt = "not base64!!!!".to_string();

    let error = derive_secret_key("password", &kdf).expect_err("invalid salt");

    assert!(error.contains("decode encrypted secret salt failed"));
}

#[test]
fn derive_secret_key_rejects_invalid_salt_length() {
    let mut kdf = test_kdf_config();
    kdf.salt = encode_secret_bytes(&[1_u8; SECRET_SALT_LEN - 1]);

    let error = derive_secret_key("password", &kdf).expect_err("short salt");

    assert_eq!(
        error,
        format!(
            "encrypted secret salt has invalid length {}",
            SECRET_SALT_LEN - 1
        )
    );
}

#[test]
fn derive_secret_key_rejects_unbounded_memory_parameter() {
    let mut kdf = test_kdf_config();
    kdf.memory_kib = MAX_ARGON2_MEMORY_KIB + 1;

    let error = derive_secret_key("password", &kdf).expect_err("memory too high");

    assert!(error.contains("secret KDF memory_kib"));
    assert!(error.contains("outside supported range"));
}

#[test]
fn derive_secret_key_rejects_unbounded_iteration_parameter() {
    let mut kdf = test_kdf_config();
    kdf.iterations = MAX_ARGON2_ITERATIONS + 1;

    let error = derive_secret_key("password", &kdf).expect_err("iterations too high");

    assert!(error.contains("secret KDF iterations"));
    assert!(error.contains("outside supported range"));
}

#[test]
fn derive_secret_key_rejects_unbounded_lanes_parameter() {
    let mut kdf = test_kdf_config();
    kdf.lanes = MAX_ARGON2_LANES + 1;

    let error = derive_secret_key("password", &kdf).expect_err("lanes too high");

    assert!(error.contains("secret KDF lanes"));
    assert!(error.contains("outside supported range"));
}
