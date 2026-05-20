mod crypto;
mod keychain;
mod model;
mod warnings;

pub use crypto::{decrypt_secrets, encrypt_secrets};
pub use keychain::{
    clear_global_secrets, clear_profile_secrets, load_global_hydromancer_secret,
    load_global_hyperdash_secret, load_keychain_secret_payload, load_profile_hydromancer_secret,
    load_profile_secrets, store_keychain_secrets, store_secret_payload,
};
pub use model::{EncryptedSecretsConfig, SecretPayload};
pub use warnings::take_secret_warnings;

pub(crate) use warnings::push_secret_warning;

#[cfg(test)]
pub(crate) use model::SecretKdfConfig;
