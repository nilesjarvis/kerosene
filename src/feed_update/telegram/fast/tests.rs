use super::*;

#[test]
fn fast_auth_signed_out_warning_status_is_sanitized() {
    assert_eq!(
        telegram_fast_signed_out_status(Some(
            "remote sign-out failed: auth_token=token-secret".to_string()
        )),
        ("Telegram fast session signed out".to_string(), false)
    );
}
