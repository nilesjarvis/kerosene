use super::super::model::{
    DEFAULT_ARGON2_ITERATIONS, DEFAULT_ARGON2_LANES, DEFAULT_ARGON2_MEMORY_KIB, SECRET_KEY_LEN,
    SECRET_SALT_LEN, SecretKdfConfig,
};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use zeroize::Zeroizing;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Secret KDF And Encoding Helpers
// ---------------------------------------------------------------------------

pub(super) fn default_secret_kdf_config(salt: &[u8]) -> SecretKdfConfig {
    SecretKdfConfig {
        algorithm: "argon2id".to_string(),
        salt: encode_secret_bytes(salt),
        memory_kib: DEFAULT_ARGON2_MEMORY_KIB,
        iterations: DEFAULT_ARGON2_ITERATIONS,
        lanes: DEFAULT_ARGON2_LANES,
    }
}

pub(super) fn encode_secret_bytes(bytes: &[u8]) -> String {
    BASE64.encode(bytes)
}

pub(super) fn decode_secret_field(field: &str, value: &str) -> Result<Vec<u8>, String> {
    BASE64
        .decode(value)
        .map_err(|e| format!("decode encrypted secret {field} failed: {e}"))
}

const MIN_ARGON2_MEMORY_KIB: u32 = 64;
const MAX_ARGON2_MEMORY_KIB: u32 = 256 * 1024;
const MIN_ARGON2_ITERATIONS: u32 = 1;
const MAX_ARGON2_ITERATIONS: u32 = 10;
const MIN_ARGON2_LANES: u32 = 1;
const MAX_ARGON2_LANES: u32 = 8;

fn validate_secret_kdf_parameters(kdf: &SecretKdfConfig, salt_len: usize) -> Result<(), String> {
    if salt_len != SECRET_SALT_LEN {
        return Err(format!(
            "encrypted secret salt has invalid length {salt_len}"
        ));
    }
    if !(MIN_ARGON2_MEMORY_KIB..=MAX_ARGON2_MEMORY_KIB).contains(&kdf.memory_kib) {
        return Err(format!(
            "secret KDF memory_kib {} is outside supported range {}..={}",
            kdf.memory_kib, MIN_ARGON2_MEMORY_KIB, MAX_ARGON2_MEMORY_KIB
        ));
    }
    if !(MIN_ARGON2_ITERATIONS..=MAX_ARGON2_ITERATIONS).contains(&kdf.iterations) {
        return Err(format!(
            "secret KDF iterations {} is outside supported range {}..={}",
            kdf.iterations, MIN_ARGON2_ITERATIONS, MAX_ARGON2_ITERATIONS
        ));
    }
    if !(MIN_ARGON2_LANES..=MAX_ARGON2_LANES).contains(&kdf.lanes) {
        return Err(format!(
            "secret KDF lanes {} is outside supported range {}..={}",
            kdf.lanes, MIN_ARGON2_LANES, MAX_ARGON2_LANES
        ));
    }

    Ok(())
}

pub(super) fn validate_secret_kdf_config(kdf: &SecretKdfConfig) -> Result<(), String> {
    if kdf.algorithm != "argon2id" {
        return Err("unsupported secret KDF".to_string());
    }

    let salt = decode_secret_field("salt", &kdf.salt)?;
    validate_secret_kdf_parameters(kdf, salt.len())
}

pub(super) fn derive_secret_key(
    password: &str,
    kdf: &SecretKdfConfig,
) -> Result<Zeroizing<[u8; 32]>, String> {
    if password.is_empty() {
        return Err("credential password is required".to_string());
    }

    validate_secret_kdf_config(kdf)?;
    let salt = decode_secret_field("salt", &kdf.salt)?;
    let params = Params::new(
        kdf.memory_kib,
        kdf.iterations,
        kdf.lanes,
        Some(SECRET_KEY_LEN),
    )
    .map_err(|e| format!("invalid secret KDF parameters: {e}"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0_u8; SECRET_KEY_LEN]);
    argon2
        .hash_password_into(password.as_bytes(), &salt, key.as_mut())
        .map_err(|e| format!("derive credential key failed: {e}"))?;

    Ok(key)
}
