mod crypto;
mod keychain;
mod model;
mod warnings;

pub use crypto::{decrypt_secrets, encrypt_secrets};
pub use keychain::{
    clear_all_keychain_secrets, clear_keychain_secret_payload,
    clear_legacy_keychain_entries_for_payload, clear_profile_secrets, load_global_secrets,
    load_keychain_secret_payload, load_profile_secrets,
    store_keychain_secrets_with_profile_removals_with_x, store_keychain_secrets_with_x,
    store_secret_payload,
};
pub use model::{EncryptedSecretsConfig, SecretPayload};
pub use warnings::take_secret_warnings;

pub(crate) use keychain::clear_profile_secrets_by_id;
pub(crate) use warnings::push_secret_warning;

pub(crate) use crypto::validate_encrypted_secrets_metadata;

#[cfg(test)]
pub(crate) use model::SecretKdfConfig;
#[cfg(test)]
pub(crate) use warnings::secret_warning_test_lock;
