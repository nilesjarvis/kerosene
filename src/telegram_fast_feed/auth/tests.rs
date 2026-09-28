use super::*;
use grammers_client::tl;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn placeholder_pending_auths(entries: &[(&str, u64)]) -> HashMap<PendingAuthKey, PendingAuth> {
    entries
        .iter()
        .map(|(path, request_id)| ((PathBuf::from(path), *request_id), PendingAuth::Placeholder))
        .collect()
}

#[test]
fn login_code_keeps_pending_password_challenge() {
    // Isolate the lock regression so a deadlock fails with a deadline instead
    // of holding the shared registry and blocking the rest of the test suite.
    const CHILD_ENV: &str = "KEROSENE_TELEGRAM_AUTH_LOCK_TEST_CHILD";
    if std::env::var_os(CHILD_ENV).is_none() {
        let mut child = Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "telegram_fast_feed::auth::tests::login_code_keeps_pending_password_challenge",
                "--nocapture",
            ])
            .env(CHILD_ENV, "1")
            .stdout(Stdio::null())
            .spawn()
            .expect("auth regression child starts");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().expect("auth regression child status") {
                assert!(status.success(), "auth regression child failed: {status}");
                return;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("auth request blocked while restoring a password challenge");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let path = telegram_fast_session_path().expect("test config directory");
    let key = (path.clone(), 17);
    let other_request = (path, 18);
    let other_session = (PathBuf::from("/tmp/kerosene-other-session.session"), 17);
    let password = Box::new(PasswordToken::new(tl::types::account::Password {
        has_recovery: false,
        has_secure_values: false,
        has_password: true,
        current_algo: None,
        srp_b: None,
        srp_id: None,
        hint: Some("test hint".to_string()),
        email_unconfirmed_pattern: None,
        new_algo: tl::enums::PasswordKdfAlgo::Unknown,
        new_secure_algo: tl::enums::SecurePasswordKdfAlgo::Unknown,
        secure_random: Vec::new(),
        pending_reset_date: None,
        login_email_pattern: None,
    }));
    let password_identity = std::ptr::from_ref(password.as_ref());
    {
        let mut pending = pending_auths().lock().expect("pending auth registry");
        pending.clear();
        pending.insert(key.clone(), PendingAuth::Password(password));
        pending.insert(other_request.clone(), PendingAuth::Placeholder);
        pending.insert(other_session.clone(), PendingAuth::Placeholder);
    }

    let result = futures::executor::block_on(submit_telegram_fast_login_code(
        1,
        17,
        19,
        Zeroizing::new("12345".to_string()),
    ));

    assert_eq!(result, Err("Enter the Telegram 2FA password".to_string()));
    let mut pending = pending_auths().lock().expect("pending auth registry");
    assert_eq!(pending.len(), 3);
    let Some(PendingAuth::Password(password)) = pending.get(&key) else {
        panic!("password challenge should retain its original session and request");
    };
    assert_eq!(std::ptr::from_ref(password.as_ref()), password_identity);
    assert_eq!(password.hint(), Some("test hint"));
    assert!(pending.contains_key(&other_request));
    assert!(pending.contains_key(&other_session));
    pending.clear();
}

#[test]
fn clear_pending_auth_drops_abandoned_challenges() {
    let mut pending = placeholder_pending_auths(&[
        ("/tmp/kerosene-telegram-a.session", 1),
        ("/tmp/kerosene-telegram-b.session", 2),
    ]);

    assert_eq!(clear_pending_auth_map(&mut pending), 2);
    assert_eq!(clear_pending_auth_map(&mut pending), 0);
}

#[test]
fn clear_pending_auth_for_request_drops_only_matching_challenges() {
    let mut pending = placeholder_pending_auths(&[
        ("/tmp/kerosene-telegram-a.session", 1),
        ("/tmp/kerosene-telegram-b.session", 2),
    ]);

    assert_eq!(clear_pending_auth_map_for_request(&mut pending, 1), 1);
    assert!(pending.contains_key(&(PathBuf::from("/tmp/kerosene-telegram-b.session"), 2)));
    assert_eq!(clear_pending_auth_map(&mut pending), 1);
}

#[test]
fn clear_pending_auth_except_request_drops_abandoned_challenges() {
    let mut pending = placeholder_pending_auths(&[
        ("/tmp/kerosene-telegram-a.session", 1),
        ("/tmp/kerosene-telegram-a.session", 2),
        ("/tmp/kerosene-telegram-b.session", 3),
    ]);

    assert_eq!(clear_pending_auth_map_except_request(&mut pending, 2), 2);
    assert_eq!(pending.len(), 1);
    assert!(pending.contains_key(&(PathBuf::from("/tmp/kerosene-telegram-a.session"), 2)));
    assert_eq!(clear_pending_auth_map_except_request(&mut pending, 2), 0);
}

#[test]
fn sign_out_outcome_fails_when_local_session_clear_fails() {
    let result = telegram_fast_sign_out_outcome(
        Ok(()),
        Err("remove <config-dir>/telegram_fast.session failed: denied".to_string()),
    );

    let error = result.expect_err("local session clear failure should fail sign-out");
    assert!(error.starts_with(TELEGRAM_FAST_SESSION_CLEAR_FAILED));
    assert!(error.contains("denied"));
}

#[test]
fn sign_out_outcome_warns_when_remote_sign_out_fails_but_local_session_clears() {
    let result = telegram_fast_sign_out_outcome(Err("network unavailable".to_string()), Ok(1))
        .expect("local session clear should complete sign-out");

    assert_eq!(
        result,
        TelegramFastAuthOutcome::SignedOut {
            warning: Some(TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED.to_string())
        }
    );
}
