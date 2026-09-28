use super::*;

fn placeholder_pending_auths(entries: &[(&str, u64)]) -> HashMap<PendingAuthKey, PendingAuth> {
    entries
        .iter()
        .map(|(path, request_id)| ((PathBuf::from(path), *request_id), PendingAuth::Placeholder))
        .collect()
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
