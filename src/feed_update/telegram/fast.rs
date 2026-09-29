use crate::app_state::TradingTerminal;
use crate::message::{Message, TelegramFastAuthMessageResult};
use crate::telegram_fast_feed::{
    TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED, TELEGRAM_FAST_SESSION_CLEAR_FAILED,
    bundled_telegram_api_hash, bundled_telegram_api_id, clear_telegram_fast_pending_auth,
    clear_telegram_fast_pending_auth_except_request, clear_telegram_fast_pending_auth_for_request,
    request_telegram_fast_login_code, sign_out_telegram_fast, submit_telegram_fast_login_code,
    submit_telegram_fast_password,
};
use crate::telegram_feed::{TelegramFastAuthOutcome, TelegramFastAuthStage, TelegramFastFeedEvent};
use iced::Task;
use std::future::Future;
use zeroize::{Zeroize, Zeroizing};

impl TradingTerminal {
    pub(super) fn toggle_telegram_fast_feed(&mut self) -> Task<Message> {
        self.telegram_feed.fast_mode_enabled = !self.telegram_feed.fast_mode_enabled;
        self.telegram_feed.fast_connected = false;
        self.telegram_feed.clear_fast_connection_event();
        self.telegram_feed.fast_reconnect_nonce =
            self.telegram_feed.fast_reconnect_nonce.saturating_add(1);
        if self.telegram_feed.fast_mode_enabled {
            let has_api_id = self.telegram_fast_api_id().is_some();
            self.telegram_feed.fast_status = Some((
                if has_api_id {
                    "Fast mode enabled; checking Telegram session".to_string()
                } else {
                    "Fast mode enabled; enter Telegram API credentials".to_string()
                },
                false,
            ));
        } else {
            self.clear_abandoned_telegram_fast_auth_challenge();
            self.telegram_feed.fast_status = Some(("Fast mode disabled".to_string(), false));
            self.telegram_feed.fast_auth_in_flight = false;
            self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::Idle;
            self.telegram_feed.fast_code_sent_at_ms = None;
        }
        self.persist_config();
        Task::none()
    }

    /// Return to the Connect onboarding screen — used by the sign-in back chevron
    /// and the public-mode status chip. Tearing down an in-progress Fast Mode
    /// flow reuses the Fast toggle's off path.
    pub(super) fn show_telegram_onboarding(&mut self) -> Task<Message> {
        if self.telegram_feed.fast_mode_enabled {
            let _ = self.toggle_telegram_fast_feed();
        }
        self.telegram_feed.fast_code_sent_at_ms = None;
        self.telegram_feed.onboarding_dismissed = false;
        self.persist_config();
        Task::none()
    }

    pub(super) fn telegram_fast_api_id(&mut self) -> Option<i32> {
        let input = self.telegram_feed.fast_api_id_input.trim();
        if input.is_empty() {
            if let Some(api_id) = self.telegram_feed.fast_api_id {
                return Some(api_id);
            }
            if let Some(api_id) = bundled_telegram_api_id() {
                self.telegram_feed.fast_api_id = Some(api_id);
                self.telegram_feed.fast_api_id_input = api_id.to_string();
                self.persist_config();
                return Some(api_id);
            }
            self.telegram_feed.fast_status = Some(("Enter a Telegram API ID".to_string(), true));
            return None;
        }

        match input.parse::<i32>() {
            Ok(api_id) if api_id > 0 => {
                self.telegram_feed.fast_api_id = Some(api_id);
                self.telegram_feed.fast_api_id_input = api_id.to_string();
                self.persist_config();
                Some(api_id)
            }
            _ => {
                self.telegram_feed.fast_status = Some((
                    "Telegram API ID must be a positive number".to_string(),
                    true,
                ));
                None
            }
        }
    }

    pub(super) fn request_telegram_fast_code(&mut self) -> Task<Message> {
        if self.telegram_feed.fast_auth_in_flight {
            return Task::none();
        }
        if self.telegram_feed.signed_in() {
            self.telegram_feed.fast_status =
                Some(("Fast mode is already signed in".to_string(), false));
            return Task::none();
        }

        let Some(api_id) = self.telegram_fast_api_id() else {
            return Task::none();
        };
        let api_hash = Zeroizing::new(self.telegram_feed.fast_api_hash_input.trim().to_string());
        let api_hash = if api_hash.is_empty() {
            Zeroizing::new(bundled_telegram_api_hash().unwrap_or_default().to_string())
        } else {
            api_hash
        };
        let phone = Zeroizing::new(crate::telegram_feed::combine_telegram_phone(
            &self.telegram_feed.fast_country_code,
            &self.telegram_feed.fast_phone_input,
        ));
        self.perform_telegram_fast_auth("Requesting Telegram login code", move |request_id| {
            request_telegram_fast_login_code(api_id, request_id, api_hash, phone)
        })
    }

    pub(super) fn submit_telegram_fast_code(&mut self) -> Task<Message> {
        let Some(api_id) = self.telegram_fast_api_id() else {
            return Task::none();
        };
        let code = Zeroizing::new(self.telegram_feed.fast_code_input.trim().to_string());
        self.telegram_feed.fast_code_input.zeroize();
        let challenge_request_id = self.telegram_feed.fast_auth_request_id;
        self.perform_telegram_fast_auth("Signing in to Telegram", move |request_id| {
            submit_telegram_fast_login_code(api_id, challenge_request_id, request_id, code)
        })
    }

    pub(super) fn submit_telegram_fast_2fa_password(&mut self) -> Task<Message> {
        let Some(api_id) = self.telegram_fast_api_id() else {
            return Task::none();
        };
        let password = Zeroizing::new(self.telegram_feed.fast_password_input.trim().to_string());
        self.telegram_feed.fast_password_input.zeroize();
        let challenge_request_id = self.telegram_feed.fast_auth_request_id;
        self.perform_telegram_fast_auth("Checking Telegram 2FA password", move |request_id| {
            submit_telegram_fast_password(api_id, challenge_request_id, request_id, password)
        })
    }

    pub(super) fn sign_out_telegram_fast_feed(&mut self) -> Task<Message> {
        clear_telegram_fast_pending_auth();
        let Some(api_id) = self.telegram_fast_api_id() else {
            return Task::none();
        };
        self.telegram_feed
            .invalidate_private_channel_candidates_request();
        self.perform_telegram_fast_auth("Signing out of Telegram", move |_| {
            sign_out_telegram_fast(api_id)
        })
    }

    /// Keep the request's generation paired with its completion message.
    fn perform_telegram_fast_auth<F>(
        &mut self,
        status: &str,
        request: impl FnOnce(u64) -> F,
    ) -> Task<Message>
    where
        F: Future<Output = Result<TelegramFastAuthOutcome, String>> + Send + 'static,
    {
        let request_id = self.telegram_feed.next_fast_auth_request_id();
        self.telegram_feed.fast_auth_in_flight = true;
        self.telegram_feed.fast_status = Some((status.to_string(), false));

        Task::perform(request(request_id), move |result| {
            Message::TelegramFastAuthResult(request_id, TelegramFastAuthMessageResult::new(result))
        })
    }

    pub(super) fn handle_telegram_fast_auth_result(
        &mut self,
        request_id: u64,
        result: Result<TelegramFastAuthOutcome, String>,
    ) -> Task<Message> {
        if request_id != self.telegram_feed.fast_auth_request_id {
            clear_telegram_fast_pending_auth_for_request(request_id);
            return Task::none();
        }
        self.telegram_feed.fast_auth_in_flight = false;
        match result {
            Ok(TelegramFastAuthOutcome::CodeSent) => {
                clear_telegram_fast_pending_auth_except_request(request_id);
                self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
                self.telegram_feed.fast_code_sent_at_ms = Some(Self::now_ms());
                self.telegram_feed.fast_status = Some(("Telegram code sent".to_string(), false));
            }
            Ok(TelegramFastAuthOutcome::PasswordRequired { hint }) => {
                clear_telegram_fast_pending_auth_except_request(request_id);
                self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::PasswordRequired;
                self.telegram_feed.fast_password_hint = hint;
                self.telegram_feed.fast_status =
                    Some(("Telegram 2FA password required".to_string(), false));
            }
            Ok(TelegramFastAuthOutcome::SignedIn { display_name }) => {
                clear_telegram_fast_pending_auth();
                self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;
                self.telegram_feed.fast_connected = true;
                self.telegram_feed
                    .record_fast_connection_event(Self::now_ms());
                self.telegram_feed.fast_code_input.zeroize();
                self.telegram_feed.fast_password_input.zeroize();
                self.telegram_feed.fast_api_hash_input.zeroize();
                self.telegram_feed.fast_phone_input.clear();
                self.telegram_feed.fast_code_sent_at_ms = None;
                self.telegram_feed.fast_password_hint = None;
                self.telegram_feed.fast_reconnect_nonce =
                    self.telegram_feed.fast_reconnect_nonce.saturating_add(1);
                self.telegram_feed.fast_status =
                    Some((format!("Fast mode signed in as {display_name}"), false));
            }
            Ok(TelegramFastAuthOutcome::SignedOut { warning }) => {
                clear_telegram_fast_pending_auth();
                self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::Idle;
                self.telegram_feed.fast_connected = false;
                self.telegram_feed.clear_fast_connection_event();
                self.telegram_feed.fast_code_input.zeroize();
                self.telegram_feed.fast_password_input.zeroize();
                self.telegram_feed.fast_phone_input.clear();
                self.telegram_feed.fast_code_sent_at_ms = None;
                self.telegram_feed.fast_reconnect_nonce =
                    self.telegram_feed.fast_reconnect_nonce.saturating_add(1);
                self.telegram_feed.fast_status = Some(telegram_fast_signed_out_status(warning));
            }
            Err(err) => {
                self.telegram_feed.fast_status =
                    Some((telegram_fast_auth_error_status(&err), true));
            }
        }
        Task::none()
    }

    pub(super) fn clear_abandoned_telegram_fast_auth_challenge(&mut self) {
        clear_telegram_fast_pending_auth();
        self.telegram_feed.invalidate_fast_auth_request();
        self.telegram_feed
            .invalidate_private_channel_candidates_request();
        if matches!(
            self.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::CodeRequested | TelegramFastAuthStage::PasswordRequired
        ) {
            self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::Idle;
            self.telegram_feed.fast_password_hint = None;
            self.telegram_feed.fast_code_input.zeroize();
            self.telegram_feed.fast_password_input.zeroize();
        }
    }

    pub(super) fn handle_telegram_fast_feed_event(
        &mut self,
        reconnect_nonce: u64,
        event: TelegramFastFeedEvent,
    ) -> Task<Message> {
        if !self.telegram_feed.fast_mode_enabled
            || reconnect_nonce != self.telegram_feed.fast_reconnect_nonce
        {
            return Task::none();
        }

        match event {
            TelegramFastFeedEvent::Status {
                connected,
                auth_required,
                message,
            } => {
                self.telegram_feed.fast_connected = connected;
                if connected {
                    self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;
                    self.telegram_feed
                        .record_fast_connection_event(Self::now_ms());
                } else if auth_required {
                    self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::Idle;
                    self.telegram_feed.clear_fast_connection_event();
                    self.telegram_feed
                        .invalidate_private_channel_candidates_request();
                }
                self.telegram_feed.fast_status = Some((
                    telegram_fast_stream_status(connected, auth_required, &message),
                    auth_required,
                ));
                Task::none()
            }
            TelegramFastFeedEvent::Loaded(channel, result) => {
                self.telegram_feed
                    .record_fast_connection_event(Self::now_ms());
                self.handle_telegram_feed_loaded(channel, *result)
            }
        }
    }
}

fn telegram_fast_auth_error_status(error: &str) -> String {
    if error.starts_with(TELEGRAM_FAST_SESSION_CLEAR_FAILED) {
        return TELEGRAM_FAST_SESSION_CLEAR_FAILED.to_string();
    }

    const SAFE_MESSAGES: &[&str] = &[
        "Enter a Telegram API hash",
        "Enter a Telegram phone number",
        "Enter the Telegram login code",
        "Request a Telegram login code first",
        "Enter the Telegram 2FA password",
        "Submit the Telegram login code first",
        "No Telegram 2FA challenge is pending",
        "Telegram 2FA password was invalid",
    ];

    if SAFE_MESSAGES.contains(&error) {
        error.to_string()
    } else {
        "Telegram fast-mode request failed".to_string()
    }
}

fn telegram_fast_signed_out_status(warning: Option<String>) -> (String, bool) {
    match warning.as_deref() {
        Some(TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED) => {
            (TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED.to_string(), true)
        }
        _ => ("Telegram fast session signed out".to_string(), false),
    }
}

fn telegram_fast_stream_status(connected: bool, auth_required: bool, message: &str) -> String {
    if auth_required {
        return "Fast mode needs Telegram sign-in".to_string();
    }
    if !connected {
        return "Telegram fast feed disconnected; reconnecting".to_string();
    }

    if is_safe_telegram_fast_stream_status(message) {
        message.to_string()
    } else {
        "Fast Telegram mode listening".to_string()
    }
}

fn is_safe_telegram_fast_stream_status(message: &str) -> bool {
    matches!(
        message,
        "Fast Telegram mode resolving channels"
            | "Fast Telegram mode listening"
            | "Fast Telegram mode connected; preparing channel backfill"
            | "Telegram backfill incomplete; continuing"
    ) || message.starts_with("Fast Telegram mode listening; could not resolve ")
}

#[cfg(test)]
mod tests;
