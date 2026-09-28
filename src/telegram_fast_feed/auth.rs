use super::clear_all_fast_channel_cursors;
use super::session::{
    clear_telegram_fast_session_files, telegram_fast_session_path, with_telegram_client,
};
use crate::telegram_feed::TelegramFastAuthOutcome;
use grammers_client::SignInError;
use grammers_client::client::{LoginToken, PasswordToken};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use zeroize::Zeroizing;

pub(crate) const TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED: &str =
    "Telegram fast local session was removed, but remote sign-out could not be confirmed";
pub(crate) const TELEGRAM_FAST_SESSION_CLEAR_FAILED: &str =
    "Telegram fast session sign-out could not remove the local session files";

enum PendingAuth {
    Login(LoginToken),
    Password(Box<PasswordToken>),
    #[cfg(test)]
    Placeholder,
}
type PendingAuthKey = (PathBuf, u64);

fn pending_auths() -> &'static Mutex<HashMap<PendingAuthKey, PendingAuth>> {
    static PENDING: OnceLock<Mutex<HashMap<PendingAuthKey, PendingAuth>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn bundled_telegram_api_id() -> Option<i32> {
    option_env!("KEROSENE_TELEGRAM_API_ID").and_then(|value| value.parse::<i32>().ok())
}

pub(crate) fn bundled_telegram_api_hash() -> Option<&'static str> {
    option_env!("KEROSENE_TELEGRAM_API_HASH").filter(|value| !value.trim().is_empty())
}

pub(crate) async fn request_telegram_fast_login_code(
    api_id: i32,
    request_id: u64,
    api_hash: Zeroizing<String>,
    phone: Zeroizing<String>,
) -> Result<TelegramFastAuthOutcome, String> {
    let api_hash = Zeroizing::new(api_hash.trim().to_string());
    let phone = Zeroizing::new(phone.trim().to_string());
    if api_hash.is_empty() {
        return Err("Enter a Telegram API hash".to_string());
    }
    if phone.is_empty() {
        return Err("Enter a Telegram phone number".to_string());
    }

    let session_path = telegram_fast_session_path()
        .ok_or_else(|| "Could not resolve Kerosene config directory".to_string())?;
    with_telegram_client(api_id, |client| async move {
        if client
            .is_authorized()
            .await
            .map_err(|_| "Telegram authorization check failed".to_string())?
        {
            return Ok(TelegramFastAuthOutcome::SignedIn {
                display_name: "Telegram".to_string(),
            });
        }

        let token = client
            .request_login_code(&phone, &api_hash)
            .await
            .map_err(|_| "Telegram login code request failed".to_string())?;
        if let Ok(mut pending) = pending_auths().lock() {
            pending.insert((session_path, request_id), PendingAuth::Login(token));
        }
        Ok(TelegramFastAuthOutcome::CodeSent)
    })
    .await
}

pub(crate) async fn submit_telegram_fast_login_code(
    api_id: i32,
    challenge_request_id: u64,
    result_request_id: u64,
    code: Zeroizing<String>,
) -> Result<TelegramFastAuthOutcome, String> {
    let code = Zeroizing::new(code.trim().to_string());
    if code.is_empty() {
        return Err("Enter the Telegram login code".to_string());
    }

    let session_path = telegram_fast_session_path()
        .ok_or_else(|| "Could not resolve Kerosene config directory".to_string())?;
    let token = {
        let mut pending = pending_auths()
            .lock()
            .map_err(|_| "Telegram login state is unavailable".to_string())?;
        let key = (session_path, challenge_request_id);
        match pending.remove(&key) {
            Some(PendingAuth::Login(token)) => token,
            Some(auth @ PendingAuth::Password(_)) => {
                pending.insert(key, auth);
                return Err("Enter the Telegram 2FA password".to_string());
            }
            #[cfg(test)]
            Some(PendingAuth::Placeholder) => {
                return Err("Request a Telegram login code first".to_string());
            }
            None => return Err("Request a Telegram login code first".to_string()),
        }
    };

    with_telegram_client(api_id, |client| async move {
        match client.sign_in(&token, &code).await {
            Ok(user) => Ok(TelegramFastAuthOutcome::SignedIn {
                display_name: user.first_name().unwrap_or("Telegram").to_string(),
            }),
            Err(SignInError::PasswordRequired(password)) => {
                let hint = password.hint().map(str::to_string);
                if let Some(session_path) = telegram_fast_session_path()
                    && let Ok(mut pending) = pending_auths().lock()
                {
                    pending.insert(
                        (session_path, result_request_id),
                        PendingAuth::Password(Box::new(password)),
                    );
                }
                Ok(TelegramFastAuthOutcome::PasswordRequired { hint })
            }
            Err(_) => Err("Telegram sign-in failed".to_string()),
        }
    })
    .await
}

pub(crate) async fn submit_telegram_fast_password(
    api_id: i32,
    challenge_request_id: u64,
    result_request_id: u64,
    password: Zeroizing<String>,
) -> Result<TelegramFastAuthOutcome, String> {
    if password.trim().is_empty() {
        return Err("Enter the Telegram 2FA password".to_string());
    }

    let session_path = telegram_fast_session_path()
        .ok_or_else(|| "Could not resolve Kerosene config directory".to_string())?;
    let token = {
        let mut pending = pending_auths()
            .lock()
            .map_err(|_| "Telegram login state is unavailable".to_string())?;
        let key = (session_path, challenge_request_id);
        match pending.remove(&key) {
            Some(PendingAuth::Password(token)) => *token,
            Some(auth @ PendingAuth::Login(_)) => {
                pending.insert(key, auth);
                return Err("Submit the Telegram login code first".to_string());
            }
            #[cfg(test)]
            Some(PendingAuth::Placeholder) => {
                return Err("No Telegram 2FA challenge is pending".to_string());
            }
            None => return Err("No Telegram 2FA challenge is pending".to_string()),
        }
    };

    with_telegram_client(api_id, |client| async move {
        match client.check_password(token, password.as_bytes()).await {
            Ok(user) => Ok(TelegramFastAuthOutcome::SignedIn {
                display_name: user.first_name().unwrap_or("Telegram").to_string(),
            }),
            Err(SignInError::InvalidPassword(token)) => {
                if let Some(session_path) = telegram_fast_session_path()
                    && let Ok(mut pending) = pending_auths().lock()
                {
                    pending.insert(
                        (session_path, result_request_id),
                        PendingAuth::Password(Box::new(token)),
                    );
                }
                Err("Telegram 2FA password was invalid".to_string())
            }
            Err(_) => Err("Telegram 2FA sign-in failed".to_string()),
        }
    })
    .await
}

pub(crate) async fn sign_out_telegram_fast(api_id: i32) -> Result<TelegramFastAuthOutcome, String> {
    let remote_result = with_telegram_client(api_id, |client| async move {
        if client
            .is_authorized()
            .await
            .map_err(|_| "Telegram authorization check failed".to_string())?
        {
            client
                .sign_out()
                .await
                .map_err(|_| "Telegram remote sign-out failed".to_string())?;
        }
        Ok(())
    })
    .await;
    clear_telegram_fast_pending_auth();
    let result = telegram_fast_sign_out_outcome(remote_result, clear_telegram_fast_session_files());
    if result.is_ok() {
        clear_all_fast_channel_cursors().await;
    }
    result
}

fn telegram_fast_sign_out_outcome(
    remote_result: Result<(), String>,
    session_clear_result: Result<usize, String>,
) -> Result<TelegramFastAuthOutcome, String> {
    if let Err(error) = session_clear_result {
        return Err(format!("{TELEGRAM_FAST_SESSION_CLEAR_FAILED}: {error}"));
    }

    Ok(TelegramFastAuthOutcome::SignedOut {
        warning: remote_result
            .err()
            .map(|_| TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED.to_string()),
    })
}

pub(crate) fn clear_telegram_fast_pending_auth() -> usize {
    if let Ok(mut pending) = pending_auths().lock() {
        clear_pending_auth_map(&mut pending)
    } else {
        0
    }
}

pub(crate) fn clear_telegram_fast_pending_auth_for_request(request_id: u64) -> usize {
    if let Ok(mut pending) = pending_auths().lock() {
        clear_pending_auth_map_for_request(&mut pending, request_id)
    } else {
        0
    }
}

pub(crate) fn clear_telegram_fast_pending_auth_except_request(request_id: u64) -> usize {
    if let Ok(mut pending) = pending_auths().lock() {
        clear_pending_auth_map_except_request(&mut pending, request_id)
    } else {
        0
    }
}

fn clear_pending_auth_map(pending: &mut HashMap<PendingAuthKey, PendingAuth>) -> usize {
    let cleared = pending.len();
    pending.clear();
    cleared
}

fn clear_pending_auth_map_for_request(
    pending: &mut HashMap<PendingAuthKey, PendingAuth>,
    request_id: u64,
) -> usize {
    let original_len = pending.len();
    pending.retain(|(_, pending_request_id), _| *pending_request_id != request_id);
    original_len.saturating_sub(pending.len())
}

fn clear_pending_auth_map_except_request(
    pending: &mut HashMap<PendingAuthKey, PendingAuth>,
    request_id: u64,
) -> usize {
    let original_len = pending.len();
    pending.retain(|(_, pending_request_id), _| *pending_request_id == request_id);
    original_len.saturating_sub(pending.len())
}

#[cfg(test)]
pub(crate) fn telegram_fast_pending_auth_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
pub(crate) fn set_telegram_fast_pending_auth_placeholders_for_test(entries: &[(&str, u64)]) {
    if let Ok(mut pending) = pending_auths().lock() {
        pending.clear();
        pending.extend(entries.iter().map(|(path, request_id)| {
            ((PathBuf::from(path), *request_id), PendingAuth::Placeholder)
        }));
    }
}

#[cfg(test)]
pub(crate) fn telegram_fast_pending_auth_request_ids_for_test() -> Vec<u64> {
    let Ok(pending) = pending_auths().lock() else {
        return Vec::new();
    };
    let mut ids = pending
        .keys()
        .map(|(_, request_id)| *request_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

#[cfg(test)]
mod tests;
