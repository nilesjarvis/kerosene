use crate::config::AccountProfile;

use super::*;

fn test_profile() -> AccountProfile {
    AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Main".to_string(),
        wallet_address: "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        agent_key: "agent-secret".to_string().into(),
        hydromancer_api_key: String::new().into(),
    }
}

fn test_kdf_config() -> SecretKdfConfig {
    let mut kdf = default_secret_kdf_config(&[7_u8; SECRET_SALT_LEN]);
    kdf.memory_kib = 64;
    kdf.iterations = 1;
    kdf.lanes = 1;
    kdf
}

#[test]
fn encrypted_secrets_round_trip_with_password() {
    let profiles = vec![test_profile()];
    let payload =
        SecretPayload::from_credentials(&profiles, "hydro-secret", "hyper-secret", "x-secret");

    let encrypted = encrypt_secrets_with_kdf(&payload, "correct horse", test_kdf_config())
        .expect("secrets should encrypt");
    let decrypted = decrypt_secrets(&encrypted, "correct horse").expect("password should decrypt");

    assert_eq!(decrypted, payload);
}

#[test]
fn encrypted_secrets_reject_wrong_password() {
    let profiles = vec![AccountProfile {
        wallet_address: String::new(),
        ..test_profile()
    }];
    let payload = SecretPayload::from_credentials(&profiles, "", "", "");
    let encrypted = encrypt_secrets_with_kdf(&payload, "right", test_kdf_config())
        .expect("secrets should encrypt");

    let error = decrypt_secrets(&encrypted, "wrong").expect_err("wrong password should fail");

    assert!(error.contains("password may be incorrect"));
}

fn test_encrypted_config() -> EncryptedSecretsConfig {
    let profiles = vec![test_profile()];
    let payload = SecretPayload::from_credentials(&profiles, "", "", "");
    encrypt_secrets_with_kdf(&payload, "password", test_kdf_config()).expect("encrypt fixture")
}

#[test]
fn encrypted_secret_metadata_accepts_valid_config_without_password() {
    let encrypted = test_encrypted_config();

    validate_encrypted_secrets_metadata(&encrypted).expect("valid metadata");
}

#[test]
fn encrypted_secret_metadata_rejects_unsupported_version() {
    let mut encrypted = test_encrypted_config();
    encrypted.version = ENCRYPTED_SECRETS_VERSION + 1;

    let error = validate_encrypted_secrets_metadata(&encrypted).expect_err("unsupported version");

    assert!(error.contains("unsupported encrypted secrets version"));
    assert!(!error.contains(&(ENCRYPTED_SECRETS_VERSION + 1).to_string()));
}

#[test]
fn encrypted_secret_metadata_rejects_unsupported_cipher_without_echoing_value() {
    let mut encrypted = test_encrypted_config();
    encrypted.cipher = "cipher-name-with-sensitive-diagnostic-marker".to_string();

    let error = validate_encrypted_secrets_metadata(&encrypted).expect_err("unsupported cipher");

    assert_eq!(error, "unsupported encrypted secrets cipher");
    assert!(!error.contains("sensitive-diagnostic-marker"));
}

#[test]
fn encrypted_secret_metadata_rejects_invalid_nonce_encoding() {
    let mut encrypted = test_encrypted_config();
    encrypted.nonce = "not base64!!!!".to_string();

    let error = validate_encrypted_secrets_metadata(&encrypted).expect_err("invalid nonce");

    assert!(error.contains("decode encrypted secret nonce failed"));
}

#[test]
fn encrypted_secret_metadata_rejects_invalid_kdf_bounds() {
    let mut encrypted = test_encrypted_config();
    encrypted.kdf.memory_kib = 1;

    let error = validate_encrypted_secrets_metadata(&encrypted).expect_err("invalid kdf");

    assert!(error.contains("secret KDF memory_kib"));
    assert!(error.contains("outside supported range"));
}
