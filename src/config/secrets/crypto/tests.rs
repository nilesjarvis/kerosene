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
    let payload = SecretPayload::from_credentials(&profiles, "hydro-secret", "hyper-secret");

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
    let payload = SecretPayload::from_credentials(&profiles, "", "");
    let encrypted = encrypt_secrets_with_kdf(&payload, "right", test_kdf_config())
        .expect("secrets should encrypt");

    let error = decrypt_secrets(&encrypted, "wrong").expect_err("wrong password should fail");

    assert!(error.contains("password may be incorrect"));
}
