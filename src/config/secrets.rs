mod crypto;
mod keychain;
mod model;
mod warnings;

pub use crypto::{decrypt_secrets, encrypt_secrets};
pub use keychain::{
    clear_global_secrets, clear_profile_hydromancer_secret, clear_profile_secrets,
    load_global_hydromancer_secret, load_global_hyperdash_secret, load_profile_secrets,
    store_global_hydromancer_secret, store_global_hyperdash_secret, store_profile_secrets,
};
pub use model::{EncryptedSecretsConfig, SecretPayload};
pub use warnings::take_secret_warnings;

pub(crate) use warnings::push_secret_warning;

#[cfg(test)]
pub(crate) use model::SecretKdfConfig;
