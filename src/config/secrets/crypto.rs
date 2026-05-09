use super::model::{
    ENCRYPTED_SECRETS_CIPHER, ENCRYPTED_SECRETS_VERSION, EncryptedSecretsConfig, SECRET_NONCE_LEN,
    SECRET_PAYLOAD_SCHEMA, SECRET_SALT_LEN, SecretKdfConfig, SecretPayload,
};

use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, rand_core::RngCore},
};
use zeroize::Zeroizing;

mod kdf;

#[cfg(test)]
mod tests;

use kdf::{decode_secret_field, default_secret_kdf_config, derive_secret_key, encode_secret_bytes};

fn encrypt_secrets_with_kdf(
    payload: &SecretPayload,
    password: &str,
    kdf: SecretKdfConfig,
) -> Result<EncryptedSecretsConfig, String> {
    let mut nonce = [0_u8; SECRET_NONCE_LEN];
    chacha20poly1305::aead::OsRng.fill_bytes(&mut nonce);

    let key = derive_secret_key(password, &kdf)?;
    let cipher = XChaCha20Poly1305::new_from_slice(key.as_ref())
        .map_err(|e| format!("initialize credential cipher failed: {e}"))?;
    let plaintext = Zeroizing::new(
        serde_json::to_vec(payload).map_err(|e| format!("serialize secrets failed: {e}"))?,
    );
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext.as_ref())
        .map_err(|e| format!("encrypt secrets failed: {e}"))?;

    Ok(EncryptedSecretsConfig {
        version: ENCRYPTED_SECRETS_VERSION,
        kdf,
        cipher: ENCRYPTED_SECRETS_CIPHER.to_string(),
        nonce: encode_secret_bytes(&nonce),
        ciphertext: encode_secret_bytes(&ciphertext),
    })
}

pub fn encrypt_secrets(
    payload: &SecretPayload,
    password: &str,
) -> Result<EncryptedSecretsConfig, String> {
    let mut salt = [0_u8; SECRET_SALT_LEN];
    chacha20poly1305::aead::OsRng.fill_bytes(&mut salt);
    encrypt_secrets_with_kdf(payload, password, default_secret_kdf_config(&salt))
}

pub fn decrypt_secrets(
    encrypted: &EncryptedSecretsConfig,
    password: &str,
) -> Result<SecretPayload, String> {
    if encrypted.version != ENCRYPTED_SECRETS_VERSION {
        return Err(format!(
            "unsupported encrypted secrets version {}",
            encrypted.version
        ));
    }
    if encrypted.cipher != ENCRYPTED_SECRETS_CIPHER {
        return Err(format!(
            "unsupported encrypted secrets cipher '{}'",
            encrypted.cipher
        ));
    }

    let nonce = decode_secret_field("nonce", &encrypted.nonce)?;
    if nonce.len() != SECRET_NONCE_LEN {
        return Err(format!(
            "encrypted secret nonce has invalid length {}",
            nonce.len()
        ));
    }
    let ciphertext = decode_secret_field("ciphertext", &encrypted.ciphertext)?;
    let key = derive_secret_key(password, &encrypted.kdf)?;
    let cipher = XChaCha20Poly1305::new_from_slice(key.as_ref())
        .map_err(|e| format!("initialize credential cipher failed: {e}"))?;
    let plaintext = Zeroizing::new(
        cipher
            .decrypt(XNonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| "decrypt secrets failed; password may be incorrect".to_string())?,
    );
    let payload: SecretPayload = serde_json::from_slice(plaintext.as_ref())
        .map_err(|e| format!("parse decrypted secrets failed: {e}"))?;
    if payload.schema != SECRET_PAYLOAD_SCHEMA {
        return Err(format!(
            "unsupported secret payload schema '{}'",
            payload.schema
        ));
    }

    Ok(payload)
}
