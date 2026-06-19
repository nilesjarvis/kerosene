use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::{ellipsized_text, redact_sensitive_response_text};
use crate::message::{Message, TelegramFastAuthMessageResult};
use crate::telegram_fast_feed::{
    TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED, TELEGRAM_FAST_SESSION_CLEAR_FAILED,
    bundled_telegram_api_hash, bundled_telegram_api_id, clear_telegram_fast_pending_auth,
    clear_telegram_fast_pending_auth_except_request, clear_telegram_fast_pending_auth_for_request,
    list_telegram_private_channel_candidates, request_telegram_fast_login_code,
    sign_out_telegram_fast, submit_telegram_fast_login_code, submit_telegram_fast_password,
};
use crate::telegram_feed::{
    TELEGRAM_AVATAR_RETRY_BACKOFF_MS, TELEGRAM_FEED_MAX_PUBLIC_CHANNELS, TelegramFastAuthOutcome,
    TelegramFastAuthStage, TelegramFastFeedEvent, TelegramFeedPage, TelegramFeedPost,
    TelegramFeedPostSource, TelegramTickerMention, fetch_telegram_avatar_bytes,
    fetch_telegram_channel_posts, normalize_public_channel_input, normalized_channel_list,
    telegram_private_channel_peer_id_from_key,
};
use iced::Task;
use iced::widget::image::Handle as ImageHandle;
use zeroize::{Zeroize, Zeroizing};

/// When no recorded mid exists at a post's publication time, the current live mid
/// is only an honest baseline if the post is this recent; older posts get no
/// price-impact percentage rather than one anchored to a late price.
const TELEGRAM_REFERENCE_FALLBACK_MAX_AGE_MS: u64 = 90_000;

impl TradingTerminal {
    pub(crate) fn update_telegram_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshTelegramFeed => self.request_telegram_feed_refresh(),
            Message::TelegramFeedRefreshTick => self.request_telegram_feed_background_refresh(),
            Message::TelegramFeedLoaded(channel, request_id, result) => {
                self.handle_telegram_public_feed_loaded(channel, request_id, *result)
            }
            Message::TelegramAvatarLoaded(channel, avatar_url, request_id, result) => {
                self.handle_telegram_avatar_loaded(channel, avatar_url, request_id, *result);
                Task::none()
            }
            Message::ToggleTelegramFastFeed => self.toggle_telegram_fast_feed(),
            Message::TelegramFastApiIdChanged(input) => {
                self.clear_abandoned_telegram_fast_auth_challenge();
                self.telegram_feed.fast_api_id_input = input.into_zeroizing().to_string();
                Task::none()
            }
            Message::TelegramFastApiHashChanged(input) => {
                self.clear_abandoned_telegram_fast_auth_challenge();
                self.telegram_feed.fast_api_hash_input.zeroize();
                self.telegram_feed.fast_api_hash_input = input.into_zeroizing().into();
                Task::none()
            }
            Message::TelegramFastPhoneChanged(input) => {
                self.clear_abandoned_telegram_fast_auth_challenge();
                self.telegram_feed.fast_phone_input.zeroize();
                self.telegram_feed.fast_phone_input = input.into_string();
                Task::none()
            }
            Message::TelegramFastCodeChanged(input) => {
                self.telegram_feed.fast_code_input.zeroize();
                self.telegram_feed.fast_code_input = input.into_zeroizing().into();
                Task::none()
            }
            Message::TelegramFastPasswordChanged(input) => {
                self.telegram_feed.fast_password_input.zeroize();
                self.telegram_feed.fast_password_input = input.into_zeroizing().into();
                Task::none()
            }
            Message::TelegramFastRequestCode => self.request_telegram_fast_code(),
            Message::TelegramFastSubmitCode => self.submit_telegram_fast_code(),
            Message::TelegramFastSubmitPassword => self.submit_telegram_fast_2fa_password(),
            Message::TelegramFastSignOut => self.sign_out_telegram_fast_feed(),
            Message::TelegramFastAuthResult(request_id, result) => {
                self.handle_telegram_fast_auth_result(request_id, result.into_result())
            }
            Message::TelegramFastFeedEvent(reconnect_nonce, event) => {
                self.handle_telegram_fast_feed_event(reconnect_nonce, event)
            }
            Message::TelegramFeedChannelInputChanged(input) => {
                self.telegram_feed.channel_input = input;
                Task::none()
            }
            Message::TelegramFeedAddChannel => self.add_telegram_feed_channel(),
            Message::TelegramPrivateChannelsRefresh => self.request_telegram_private_channels(),
            Message::TelegramPrivateChannelsLoaded(request_id, result) => {
                self.handle_telegram_private_channels_loaded(request_id, *result);
                Task::none()
            }
            Message::TelegramFeedAddPrivateChannel(peer_id) => {
                self.add_telegram_private_channel(peer_id);
                Task::none()
            }
            Message::ToggleTelegramPrivateChannelCandidatesExpanded => {
                self.telegram_feed.private_channel_candidates_expanded =
                    !self.telegram_feed.private_channel_candidates_expanded;
                Task::none()
            }
            Message::TelegramFeedRemoveChannel(channel) => {
                let channel = channel.into_string();
                self.remove_telegram_feed_channel(&channel);
                Task::none()
            }
            Message::ToggleTelegramFeedChannelsExpanded => {
                self.telegram_feed.channels_expanded = !self.telegram_feed.channels_expanded;
                Task::none()
            }
            Message::ToggleTelegramFeedNotifications => {
                self.telegram_feed.notifications_enabled =
                    !self.telegram_feed.notifications_enabled;
                self.persist_config();
                Task::none()
            }
            Message::ToggleTelegramFeedOutcomeMarkets => {
                // Display-time filter only, so the toggle is instant and never
                // drops already-captured outcome mentions or their references.
                self.telegram_feed.include_outcome_markets =
                    !self.telegram_feed.include_outcome_markets;
                self.persist_config();
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn request_telegram_feed_refresh(&mut self) -> Task<Message> {
        self.request_telegram_feed_refresh_with_visibility(true)
    }

    pub(crate) fn request_telegram_feed_background_refresh(&mut self) -> Task<Message> {
        if self.telegram_feed.fast_mode_enabled && self.telegram_feed.fast_connected {
            let now_ms = Self::now_ms();
            if !self.telegram_feed.fast_connection_stale(now_ms) {
                return Task::none();
            }

            self.telegram_feed.fast_connected = false;
            self.telegram_feed.fast_reconnect_nonce =
                self.telegram_feed.fast_reconnect_nonce.saturating_add(1);
            self.telegram_feed.fast_status = Some((
                "Fast Telegram mode is stale; reconnecting and using public refresh".to_string(),
                true,
            ));
        }

        // The tick subscription stays alive while a refresh is in flight (it
        // also drives the staleness check above), so skip duplicate fetches
        // here instead of gating the timer.
        if self.telegram_feed.channel_refresh_in_flight() {
            return Task::none();
        }

        self.request_telegram_feed_refresh_with_visibility(false)
    }

    fn request_telegram_feed_refresh_with_visibility(&mut self, visible: bool) -> Task<Message> {
        let channels = normalized_channel_list(&self.telegram_feed.channels);
        let channel_limit_warning = if channels != self.telegram_feed.channels {
            self.telegram_feed.channels = channels.clone();
            Some(format!(
                "Telegram Feed supports up to {TELEGRAM_FEED_MAX_PUBLIC_CHANNELS} public channels; extra channels were ignored"
            ))
        } else {
            None
        };
        if channels.is_empty() {
            if visible {
                self.telegram_feed.last_error =
                    Some(if self.telegram_feed.private_channels.is_empty() {
                        "Add a public Telegram channel".to_string()
                    } else {
                        "Private Telegram channels require signed-in fast mode".to_string()
                    });
            }
            return Task::none();
        }

        if visible {
            self.telegram_feed.loading_channels = channels.clone();
            self.telegram_feed.last_error = channel_limit_warning;
        } else {
            if let Some(warning) = channel_limit_warning {
                self.telegram_feed.last_error = Some(warning);
            }
            self.telegram_feed.background_loading_channels = channels.clone();
        }
        let tasks = channels
            .into_iter()
            .map(|channel| self.request_telegram_channel_refresh_task(channel))
            .collect::<Vec<_>>();
        Task::batch(tasks)
    }

    fn add_telegram_feed_channel(&mut self) -> Task<Message> {
        let input = self.telegram_feed.channel_input.clone();
        let channel = match normalize_public_channel_input(&input) {
            Ok(channel) => channel,
            Err(err) => {
                self.telegram_feed.last_error = Some(err);
                return Task::none();
            }
        };

        if self
            .telegram_feed
            .channels
            .iter()
            .any(|existing| existing == &channel)
        {
            self.telegram_feed.channel_input.clear();
            self.telegram_feed.last_error = Some(format!("@{channel} is already in the feed"));
            return Task::none();
        }

        if self.telegram_feed.channels.len() >= TELEGRAM_FEED_MAX_PUBLIC_CHANNELS {
            self.telegram_feed.last_error = Some(format!(
                "Telegram Feed supports up to {TELEGRAM_FEED_MAX_PUBLIC_CHANNELS} public channels"
            ));
            return Task::none();
        }

        crate::telegram_fast_feed::clear_fast_channel_cursor(&channel);
        self.telegram_feed.channels.push(channel.clone());
        self.telegram_feed.channel_input.clear();
        self.telegram_feed.last_error = None;
        self.restart_telegram_fast_feed_after_channel_change();
        self.persist_config();
        self.telegram_feed.loading_channels.push(channel.clone());
        self.request_telegram_channel_refresh_task(channel)
    }

    fn request_telegram_private_channels(&mut self) -> Task<Message> {
        if !self.telegram_feed.fast_mode_enabled {
            self.telegram_feed.fast_status =
                Some(("Enable fast mode to add private channels".to_string(), true));
            return Task::none();
        }
        let Some(api_id) = self.telegram_fast_api_id() else {
            return Task::none();
        };
        if !self.telegram_feed.fast_connected
            && !matches!(
                self.telegram_feed.fast_auth_stage,
                TelegramFastAuthStage::SignedIn
            )
        {
            self.telegram_feed.fast_status =
                Some(("Sign in to Telegram fast mode first".to_string(), true));
            return Task::none();
        }

        let request_id = self
            .telegram_feed
            .next_private_channel_candidates_request_id();
        self.telegram_feed.private_channel_candidates_loading = true;
        self.telegram_feed.fast_status = Some(("Scanning Telegram channels".to_string(), false));
        Task::perform(
            list_telegram_private_channel_candidates(api_id),
            move |result| Message::TelegramPrivateChannelsLoaded(request_id, Box::new(result)),
        )
    }

    fn handle_telegram_private_channels_loaded(
        &mut self,
        request_id: u64,
        result: Result<Vec<crate::telegram_feed::TelegramPrivateChannelCandidate>, String>,
    ) {
        if request_id != self.telegram_feed.private_channel_candidates_request_id {
            return;
        }
        self.telegram_feed.private_channel_candidates_loading = false;
        match result {
            Ok(candidates) => {
                let count = candidates.len();
                self.telegram_feed.private_channel_candidates = candidates;
                self.telegram_feed.private_channel_candidates_expanded = count > 0;
                self.telegram_feed.fast_status =
                    Some((format!("Found {count} private Telegram channels"), false));
            }
            Err(err) => {
                self.telegram_feed.fast_status =
                    Some((telegram_private_channel_error_status(&err), true));
            }
        }
    }

    fn add_telegram_private_channel(&mut self, peer_id: i64) {
        if self.telegram_feed.private_channel_selected(peer_id) {
            self.telegram_feed.last_error =
                Some("Private channel is already in the feed".to_string());
            return;
        }
        let Some(candidate) = self
            .telegram_feed
            .private_channel_candidates
            .iter()
            .find(|candidate| candidate.peer_id == peer_id)
            .cloned()
        else {
            self.telegram_feed.last_error = Some("Refresh private channels first".to_string());
            return;
        };

        let candidate_config = candidate.to_config();
        crate::telegram_fast_feed::clear_fast_channel_cursor(&candidate_config.key());
        self.telegram_feed.private_channels.push(candidate_config);
        let profile = candidate.to_profile();
        self.telegram_feed
            .channel_profiles
            .entry(profile.channel.clone())
            .and_modify(|existing| {
                existing.title = profile.title.clone();
                existing.initials = profile.initials.clone();
                if profile.avatar_handle.is_some() {
                    existing.avatar_handle = profile.avatar_handle.clone();
                    existing.avatar_url = None;
                    existing.avatar_loading_url = None;
                    existing.avatar_request_id = 0;
                    existing.avatar_failed_at_ms = None;
                }
            })
            .or_insert(profile);
        self.telegram_feed.last_error = None;
        self.restart_telegram_fast_feed_after_channel_change();
        self.persist_config();
    }

    fn remove_telegram_feed_channel(&mut self, channel: &str) {
        if let Some(peer_id) = telegram_private_channel_peer_id_from_key(channel) {
            self.telegram_feed
                .private_channels
                .retain(|existing| existing.peer_id != peer_id);
            self.telegram_feed
                .posts
                .retain(|post| post.channel != channel);
            self.telegram_feed.clear_seen_posts_for_channel(channel);
            crate::telegram_fast_feed::clear_fast_channel_cursor(channel);
            self.telegram_feed.channel_profiles.remove(channel);
            self.restart_telegram_fast_feed_after_channel_change();
            self.telegram_feed.last_error = None;
            self.persist_config();
            return;
        }

        let Ok(channel) = normalize_public_channel_input(channel) else {
            return;
        };
        self.telegram_feed
            .channels
            .retain(|existing| existing != &channel);
        self.telegram_feed
            .loading_channels
            .retain(|existing| existing != &channel);
        self.telegram_feed
            .background_loading_channels
            .retain(|existing| existing != &channel);
        self.telegram_feed
            .posts
            .retain(|post| post.channel != channel);
        self.telegram_feed.clear_seen_posts_for_channel(&channel);
        self.telegram_feed.clear_channel_refresh(&channel);
        crate::telegram_fast_feed::clear_fast_channel_cursor(&channel);
        self.telegram_feed.channel_profiles.remove(&channel);
        self.restart_telegram_fast_feed_after_channel_change();
        self.telegram_feed.last_error = None;
        self.persist_config();
    }

    fn restart_telegram_fast_feed_after_channel_change(&mut self) {
        self.telegram_feed.fast_connected = false;
        self.telegram_feed.clear_fast_connection_event();
        self.telegram_feed.fast_reconnect_nonce =
            self.telegram_feed.fast_reconnect_nonce.saturating_add(1);
        if self.telegram_feed.fast_mode_enabled {
            self.telegram_feed.fast_status = Some((
                "Fast Telegram mode reconnecting after channel list changed".to_string(),
                false,
            ));
        }
    }

    fn request_telegram_channel_refresh_task(&mut self, channel: String) -> Task<Message> {
        let request_id = self.telegram_feed.begin_channel_refresh(&channel);
        Task::perform(
            fetch_telegram_channel_posts(channel.clone()),
            move |result| {
                Message::TelegramFeedLoaded(channel.clone(), request_id, Box::new(result))
            },
        )
    }

    fn toggle_telegram_fast_feed(&mut self) -> Task<Message> {
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
        }
        self.persist_config();
        Task::none()
    }

    fn telegram_fast_api_id(&mut self) -> Option<i32> {
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

    fn request_telegram_fast_code(&mut self) -> Task<Message> {
        if self.telegram_feed.fast_auth_in_flight {
            return Task::none();
        }
        if self.telegram_feed.fast_connected
            || matches!(
                self.telegram_feed.fast_auth_stage,
                TelegramFastAuthStage::SignedIn
            )
        {
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
        let phone = Zeroizing::new(self.telegram_feed.fast_phone_input.trim().to_string());
        let request_id = self.telegram_feed.next_fast_auth_request_id();
        self.telegram_feed.fast_auth_in_flight = true;
        self.telegram_feed.fast_status =
            Some(("Requesting Telegram login code".to_string(), false));

        Task::perform(
            request_telegram_fast_login_code(api_id, request_id, api_hash, phone),
            move |result| {
                Message::TelegramFastAuthResult(
                    request_id,
                    TelegramFastAuthMessageResult::new(result),
                )
            },
        )
    }

    fn submit_telegram_fast_code(&mut self) -> Task<Message> {
        let Some(api_id) = self.telegram_fast_api_id() else {
            return Task::none();
        };
        let code = Zeroizing::new(self.telegram_feed.fast_code_input.trim().to_string());
        self.telegram_feed.fast_code_input.zeroize();
        let challenge_request_id = self.telegram_feed.fast_auth_request_id;
        let request_id = self.telegram_feed.next_fast_auth_request_id();
        self.telegram_feed.fast_auth_in_flight = true;
        self.telegram_feed.fast_status = Some(("Signing in to Telegram".to_string(), false));

        Task::perform(
            submit_telegram_fast_login_code(api_id, challenge_request_id, request_id, code),
            move |result| {
                Message::TelegramFastAuthResult(
                    request_id,
                    TelegramFastAuthMessageResult::new(result),
                )
            },
        )
    }

    fn submit_telegram_fast_2fa_password(&mut self) -> Task<Message> {
        let Some(api_id) = self.telegram_fast_api_id() else {
            return Task::none();
        };
        let password = Zeroizing::new(self.telegram_feed.fast_password_input.trim().to_string());
        self.telegram_feed.fast_password_input.zeroize();
        let challenge_request_id = self.telegram_feed.fast_auth_request_id;
        let request_id = self.telegram_feed.next_fast_auth_request_id();
        self.telegram_feed.fast_auth_in_flight = true;
        self.telegram_feed.fast_status =
            Some(("Checking Telegram 2FA password".to_string(), false));

        Task::perform(
            submit_telegram_fast_password(api_id, challenge_request_id, request_id, password),
            move |result| {
                Message::TelegramFastAuthResult(
                    request_id,
                    TelegramFastAuthMessageResult::new(result),
                )
            },
        )
    }

    fn sign_out_telegram_fast_feed(&mut self) -> Task<Message> {
        clear_telegram_fast_pending_auth();
        let Some(api_id) = self.telegram_fast_api_id() else {
            return Task::none();
        };
        self.telegram_feed
            .invalidate_private_channel_candidates_request();
        let request_id = self.telegram_feed.next_fast_auth_request_id();
        self.telegram_feed.fast_auth_in_flight = true;
        self.telegram_feed.fast_status = Some(("Signing out of Telegram".to_string(), false));

        Task::perform(sign_out_telegram_fast(api_id), move |result| {
            Message::TelegramFastAuthResult(request_id, TelegramFastAuthMessageResult::new(result))
        })
    }

    fn handle_telegram_fast_auth_result(
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
                self.telegram_feed.fast_status = Some(("Telegram code sent".to_string(), false));
            }
            Ok(TelegramFastAuthOutcome::PasswordRequired { hint }) => {
                clear_telegram_fast_pending_auth_except_request(request_id);
                self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::PasswordRequired;
                self.telegram_feed.fast_password_hint = hint.clone();
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

    fn clear_abandoned_telegram_fast_auth_challenge(&mut self) {
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

    fn handle_telegram_fast_feed_event(
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

    fn handle_telegram_public_feed_loaded(
        &mut self,
        channel: String,
        request_id: u64,
        result: Result<TelegramFeedPage, String>,
    ) -> Task<Message> {
        if !self
            .telegram_feed
            .finish_channel_refresh(&channel, request_id)
        {
            return Task::none();
        }

        self.handle_telegram_feed_loaded(channel, result)
    }

    fn handle_telegram_feed_loaded(
        &mut self,
        channel: String,
        result: Result<TelegramFeedPage, String>,
    ) -> Task<Message> {
        let was_visible_loading = self
            .telegram_feed
            .loading_channels
            .iter()
            .any(|loading| loading == &channel);
        self.telegram_feed
            .loading_channels
            .retain(|loading| loading != &channel);
        self.telegram_feed
            .background_loading_channels
            .retain(|loading| loading != &channel);

        if !self.telegram_feed.feed_source_selected(&channel) {
            return Task::none();
        }

        match result {
            Ok(page) => {
                let now_ms = Self::now_ms();
                let avatar_task = self.store_telegram_channel_profile(page.profile);
                self.telegram_feed.last_error = None;
                let had_seen_posts = self.telegram_feed.has_seen_posts_for_channel(&channel);
                let mut new_posts = Vec::new();
                for mut post in page.posts {
                    post.mark_applied(now_ms);
                    let already_seen = self
                        .telegram_feed
                        .record_seen_post(&channel, post.message_id);
                    if let Some(existing_index) =
                        self.telegram_feed.posts.iter().position(|existing| {
                            existing.channel == channel && existing.message_id == post.message_id
                        })
                    {
                        let previous_mentions = self.telegram_feed.posts[existing_index]
                            .ticker_mentions
                            .clone();
                        let mentions = self.telegram_ticker_mentions_for_text(
                            &post.text,
                            post.timestamp_ms,
                            now_ms,
                            &previous_mentions,
                        );
                        let existing_post = &mut self.telegram_feed.posts[existing_index];
                        existing_post.text = post.text;
                        existing_post.timestamp_ms = post.timestamp_ms;
                        existing_post.url = post.url;
                        existing_post.ticker_mentions = mentions;
                        existing_post.applied_at_ms = post.applied_at_ms;
                    } else {
                        // History backfill must never read as breaking news: a
                        // live fast message can land before its channel's
                        // backfill, which would otherwise flag old posts new.
                        let treat_as_new = had_seen_posts
                            && !already_seen
                            && post.source != TelegramFeedPostSource::FastBackfill;
                        if treat_as_new {
                            post.first_seen_ms = now_ms;
                        }
                        let mentions = self.telegram_ticker_mentions_for_text(
                            &post.text,
                            post.timestamp_ms,
                            now_ms,
                            &[],
                        );
                        post.ticker_mentions = mentions;
                        if treat_as_new {
                            new_posts.push(post.clone());
                        }
                        self.telegram_feed.posts.push(post);
                    }
                }
                self.telegram_feed.posts.sort_by(|left, right| {
                    right
                        .timestamp_ms
                        .cmp(&left.timestamp_ms)
                        .then_with(|| right.message_id.cmp(&left.message_id))
                        .then_with(|| left.channel.cmp(&right.channel))
                });
                self.telegram_feed.posts.dedup_by(|left, right| {
                    left.channel == right.channel && left.message_id == right.message_id
                });
                self.telegram_feed
                    .posts
                    .truncate(crate::telegram_feed::TELEGRAM_FEED_RENDER_LIMIT);
                self.telegram_feed.last_refresh_ms = Some(now_ms);
                if self.telegram_feed.notifications_enabled {
                    self.push_new_telegram_post_alerts(&new_posts);
                }
                avatar_task
            }
            Err(err) => {
                if was_visible_loading || self.telegram_feed.posts.is_empty() {
                    self.telegram_feed.last_error = Some(redact_sensitive_response_text(&err));
                }
                Task::none()
            }
        }
    }

    pub(crate) fn refresh_telegram_ticker_mentions(&mut self) {
        if self.telegram_feed.posts.is_empty() {
            return;
        }

        let now_ms = Self::now_ms();
        let mut posts = std::mem::take(&mut self.telegram_feed.posts);
        for post in &mut posts {
            let previous_mentions = post.ticker_mentions.clone();
            post.ticker_mentions = self.telegram_ticker_mentions_for_text(
                &post.text,
                post.timestamp_ms,
                now_ms,
                &previous_mentions,
            );
        }
        self.telegram_feed.posts = posts;
    }

    pub(crate) fn fill_missing_telegram_ticker_reference_prices(&mut self, now_ms: u64) {
        // Runs on every mids tick; skip the take/reinsert churn once every mention
        // already has a baseline.
        let has_missing = self.telegram_feed.posts.iter().any(|post| {
            post.ticker_mentions
                .iter()
                .any(|mention| mention.reference_price.is_none())
        });
        if !has_missing {
            return;
        }

        let mut posts = std::mem::take(&mut self.telegram_feed.posts);
        for post in &mut posts {
            for mention in &mut post.ticker_mentions {
                if mention.reference_price.is_none()
                    && let Some(price) = self.telegram_reference_price_for_mention(
                        &mention.symbol,
                        post.timestamp_ms,
                        now_ms,
                    )
                {
                    mention.reference_price = Some(price);
                    mention.reference_seen_ms = now_ms;
                }
            }
        }
        self.telegram_feed.posts = posts;
    }

    fn telegram_ticker_mentions_for_text(
        &self,
        text: &str,
        post_timestamp_ms: u64,
        now_ms: u64,
        previous_mentions: &[TelegramTickerMention],
    ) -> Vec<TelegramTickerMention> {
        self.telegram_feed
            .resolve_ticker_mentions(text)
            .into_iter()
            .filter(|matched| {
                self.resolve_exchange_symbol_by_key_or_ticker(&matched.symbol_key)
                    .is_some_and(|symbol| {
                        symbol.market_type != MarketType::Spot
                            && self.exchange_symbol_is_orderable(symbol)
                    })
            })
            .map(|matched| {
                if let Some(previous) = previous_mentions
                    .iter()
                    .find(|mention| mention.symbol == matched.symbol_key)
                {
                    let mut mention = previous.clone();
                    mention.ticker = matched.ticker;
                    mention.matched_text = matched.matched_text;
                    mention.source = matched.source;
                    mention.confidence = matched.confidence;
                    if mention.reference_price.is_none()
                        && let Some(price) = self.telegram_reference_price_for_mention(
                            &matched.symbol_key,
                            post_timestamp_ms,
                            now_ms,
                        )
                    {
                        mention.reference_price = Some(price);
                        mention.reference_seen_ms = now_ms;
                    }
                    mention
                } else {
                    let reference_price = self.telegram_reference_price_for_mention(
                        &matched.symbol_key,
                        post_timestamp_ms,
                        now_ms,
                    );
                    TelegramTickerMention {
                        reference_seen_ms: if reference_price.is_some() { now_ms } else { 0 },
                        reference_price,
                        symbol: matched.symbol_key,
                        ticker: matched.ticker,
                        matched_text: matched.matched_text,
                        source: matched.source,
                        confidence: matched.confidence,
                    }
                }
            })
            .collect()
    }

    /// The price-impact baseline for a freshly mentioned ticker. Prefers the mid
    /// recorded at or just before the message was published (the true pre-news
    /// price), so the displayed `%` measures the move *since the headline*. Falls
    /// back to the current live mid only when the post is recent enough that
    /// "now" is a faithful proxy for publication time; otherwise returns `None`
    /// so the chip shows no (misleading) percentage.
    fn telegram_reference_price_for_mention(
        &self,
        symbol: &str,
        post_timestamp_ms: u64,
        now_ms: u64,
    ) -> Option<f64> {
        let candidates = self.mid_candidates_for_symbol(symbol);
        if let Some(price) = self
            .screener
            .mid_sample_at_or_before(&candidates, post_timestamp_ms)
        {
            return Some(price);
        }
        if now_ms.saturating_sub(post_timestamp_ms) <= TELEGRAM_REFERENCE_FALLBACK_MAX_AGE_MS {
            return self.resolve_mid_for_symbol_at(symbol, now_ms);
        }
        None
    }

    fn store_telegram_channel_profile(
        &mut self,
        mut profile: crate::telegram_feed::TelegramChannelProfile,
    ) -> Task<Message> {
        let now_ms = Self::now_ms();
        if let Some(existing) = self.telegram_feed.channel_profiles.get(&profile.channel)
            && profile.avatar_url.is_none()
        {
            profile.avatar_url = existing.avatar_url.clone();
        }
        let avatar_url = profile.avatar_url.clone();
        if let Some(existing) = self
            .telegram_feed
            .channel_profiles
            .get(&profile.channel)
            .filter(|existing| existing.avatar_url == profile.avatar_url)
        {
            if profile.avatar_handle.is_none() {
                profile.avatar_handle = existing.avatar_handle.clone();
            }
            profile.avatar_loading_url = existing.avatar_loading_url.clone();
            profile.avatar_request_id = existing.avatar_request_id;
            profile.avatar_failed_at_ms = existing.avatar_failed_at_ms;
        }

        let should_fetch_avatar = avatar_url.as_ref().is_some_and(|avatar_url| {
            profile.avatar_handle.is_none()
                && profile.avatar_loading_url.as_deref() != Some(avatar_url.as_str())
                && !profile.avatar_failed_at_ms.is_some_and(|failed_at_ms| {
                    now_ms.saturating_sub(failed_at_ms) < TELEGRAM_AVATAR_RETRY_BACKOFF_MS
                })
        });
        if should_fetch_avatar {
            self.telegram_feed.next_avatar_request_id =
                self.telegram_feed.next_avatar_request_id.saturating_add(1);
            profile.avatar_loading_url = avatar_url.clone();
            profile.avatar_request_id = self.telegram_feed.next_avatar_request_id;
            profile.avatar_failed_at_ms = None;
        }

        let channel = profile.channel.clone();
        let request_id = profile.avatar_request_id;
        self.telegram_feed
            .channel_profiles
            .insert(profile.channel.clone(), profile);

        if should_fetch_avatar && let Some(avatar_url) = avatar_url {
            return Task::perform(
                fetch_telegram_avatar_bytes(channel.clone(), avatar_url.clone()),
                move |result| {
                    Message::TelegramAvatarLoaded(
                        channel.clone(),
                        avatar_url.clone(),
                        request_id,
                        Box::new(result),
                    )
                },
            );
        }

        Task::none()
    }

    fn handle_telegram_avatar_loaded(
        &mut self,
        channel: String,
        avatar_url: String,
        request_id: u64,
        result: Result<Vec<u8>, String>,
    ) {
        if !self.telegram_feed.feed_source_selected(&channel) {
            return;
        }

        if let Some(profile) = self.telegram_feed.channel_profiles.get_mut(&channel) {
            if profile.avatar_url.as_deref() != Some(avatar_url.as_str()) {
                return;
            }
            if profile.avatar_request_id != request_id {
                return;
            }
            if profile.avatar_loading_url.as_deref() == Some(avatar_url.as_str()) {
                profile.avatar_loading_url = None;
            } else {
                return;
            }

            match result {
                Ok(bytes) => {
                    profile.avatar_handle = Some(ImageHandle::from_bytes(bytes));
                    profile.avatar_request_id = 0;
                    profile.avatar_failed_at_ms = None;
                }
                Err(_) => {
                    profile.avatar_handle = None;
                    profile.avatar_request_id = 0;
                    profile.avatar_failed_at_ms = Some(Self::now_ms());
                }
            }
        }
    }

    fn push_new_telegram_post_alerts(&mut self, posts: &[TelegramFeedPost]) {
        const MAX_ALERTS_PER_REFRESH: usize = 3;

        for post in posts.iter().take(MAX_ALERTS_PER_REFRESH) {
            self.push_telegram_feed_alert(telegram_post_alert_message(post));
        }

        if posts.len() > MAX_ALERTS_PER_REFRESH {
            self.push_telegram_feed_alert(format!(
                "{} more Telegram messages",
                posts.len() - MAX_ALERTS_PER_REFRESH
            ));
        }
    }
}

fn telegram_post_alert_message(post: &TelegramFeedPost) -> String {
    const MAX_PREVIEW_CHARS: usize = 140;
    let preview = ellipsized_text(
        post.text.lines().next().unwrap_or_default(),
        MAX_PREVIEW_CHARS,
    );

    if preview.is_empty() {
        format!("@{} posted a new message", post.channel)
    } else {
        format!("@{}: {}", post.channel, preview)
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

fn telegram_private_channel_error_status(error: &str) -> String {
    const SAFE_MESSAGES: &[&str] = &[
        "Sign in to Telegram fast mode first",
        "Telegram private channel scan timed out",
    ];

    if SAFE_MESSAGES.contains(&error) {
        error.to_string()
    } else {
        "Could not scan Telegram private channels".to_string()
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
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType};
    use crate::config::KeroseneConfig;
    use crate::telegram_fast_feed::{
        fast_channel_cursor_message_id_for_test, fast_channel_cursor_test_lock,
        set_fast_channel_cursor_for_test, set_telegram_fast_pending_auth_placeholders_for_test,
        telegram_fast_pending_auth_request_ids_for_test, telegram_fast_pending_auth_test_lock,
    };

    fn exchange_symbol(key: &str, ticker: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: ticker.to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 2,
            max_leverage: 50,
            only_isolated: false,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    fn sample_post(channel: &str, message_id: u64) -> TelegramFeedPost {
        TelegramFeedPost {
            channel: channel.to_string(),
            message_id,
            text: "sample".to_string(),
            timestamp_ms: 1_000,
            source: crate::telegram_feed::TelegramFeedPostSource::PublicPoll,
            received_at_ms: 1_100,
            applied_at_ms: 0,
            fetched_at_ms: 1_100,
            request_started_ms: 1_050,
            request_duration_ms: 50,
            first_seen_ms: 0,
            url: format!("https://t.me/{channel}/{message_id}"),
            ticker_mentions: Vec::new(),
        }
    }

    fn sample_profile(
        channel: &str,
        avatar_url: Option<&str>,
    ) -> crate::telegram_feed::TelegramChannelProfile {
        crate::telegram_feed::TelegramChannelProfile {
            channel: channel.to_string(),
            title: format!("@{channel}"),
            initials: channel.chars().take(2).collect(),
            avatar_url: avatar_url.map(str::to_string),
            avatar_handle: None,
            avatar_loading_url: None,
            avatar_request_id: 0,
            avatar_failed_at_ms: None,
        }
    }

    fn sample_page(channel: &str, posts: Vec<TelegramFeedPost>) -> TelegramFeedPage {
        TelegramFeedPage {
            profile: sample_profile(channel, None),
            posts,
        }
    }

    fn load_public_feed(
        terminal: &mut TradingTerminal,
        channel: &str,
        result: Result<TelegramFeedPage, String>,
    ) {
        let request_id = terminal.telegram_feed.begin_channel_refresh(channel);
        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            channel.to_string(),
            request_id,
            Box::new(result),
        ));
    }

    fn deliver_public_feed(
        terminal: &mut TradingTerminal,
        channel: &str,
        request_id: u64,
        result: Result<TelegramFeedPage, String>,
    ) {
        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            channel.to_string(),
            request_id,
            Box::new(result),
        ));
    }

    fn deliver_enabled_fast_feed_event(
        terminal: &mut TradingTerminal,
        event: TelegramFastFeedEvent,
    ) {
        terminal.telegram_feed.fast_mode_enabled = true;
        let reconnect_nonce = terminal.telegram_feed.fast_reconnect_nonce;
        let _task =
            terminal.update_telegram_feed(Message::TelegramFastFeedEvent(reconnect_nonce, event));
    }

    fn sample_page_with_avatar(
        channel: &str,
        avatar_url: &str,
        posts: Vec<TelegramFeedPost>,
    ) -> TelegramFeedPage {
        TelegramFeedPage {
            profile: sample_profile(channel, Some(avatar_url)),
            posts,
        }
    }

    #[test]
    fn loaded_removed_channel_is_ignored() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.background_loading_channels = vec!["marketfeed".to_string()];
        let stale_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

        let _task =
            terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));
        deliver_public_feed(
            &mut terminal,
            "marketfeed",
            stale_request_id,
            Ok(sample_page(
                "marketfeed",
                vec![sample_post("marketfeed", 1)],
            )),
        );

        assert!(terminal.telegram_feed.posts.is_empty());
        assert!(
            !terminal
                .telegram_feed
                .channel_profiles
                .contains_key("marketfeed")
        );
        assert_eq!(terminal.telegram_feed.last_error, None);
        assert!(terminal.telegram_feed.loading_channels.is_empty());
        assert!(
            terminal
                .telegram_feed
                .background_loading_channels
                .is_empty()
        );
    }

    #[test]
    fn stale_public_channel_success_after_remove_and_readd_is_ignored() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
        let stale_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

        let _task =
            terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
        let _current_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

        deliver_public_feed(
            &mut terminal,
            "marketfeed",
            stale_request_id,
            Ok(sample_page(
                "marketfeed",
                vec![sample_post("marketfeed", 1)],
            )),
        );

        assert!(terminal.telegram_feed.posts.is_empty());
        assert!(
            !terminal
                .telegram_feed
                .channel_profiles
                .contains_key("marketfeed")
        );
        assert_eq!(
            terminal.telegram_feed.loading_channels,
            vec!["marketfeed".to_string()]
        );
    }

    #[test]
    fn stale_public_channel_error_after_remove_and_readd_is_ignored() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
        let stale_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

        let _task =
            terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
        let _current_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");
        terminal.telegram_feed.last_error = Some("current status".to_string());

        deliver_public_feed(
            &mut terminal,
            "marketfeed",
            stale_request_id,
            Err("stale failure".to_string()),
        );

        assert_eq!(
            terminal.telegram_feed.last_error,
            Some("current status".to_string())
        );
        assert_eq!(
            terminal.telegram_feed.loading_channels,
            vec!["marketfeed".to_string()]
        );
    }

    #[test]
    fn current_public_channel_error_redacts_last_error() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Err("telegram failed: api_hash=feed-secret".to_string()),
        );

        let error = terminal
            .telegram_feed
            .last_error
            .as_deref()
            .expect("telegram feed error");
        assert!(error.contains("api_hash=<redacted>"));
        assert!(!error.contains("feed-secret"));
    }

    #[test]
    fn current_public_channel_success_after_remove_and_readd_is_applied() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
        let _stale_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

        let _task =
            terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
        let current_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

        deliver_public_feed(
            &mut terminal,
            "marketfeed",
            current_request_id,
            Ok(sample_page(
                "marketfeed",
                vec![sample_post("marketfeed", 1)],
            )),
        );

        assert_eq!(terminal.telegram_feed.posts.len(), 1);
        assert!(
            terminal
                .telegram_feed
                .channel_profiles
                .contains_key("marketfeed")
        );
        assert!(terminal.telegram_feed.loading_channels.is_empty());
    }

    #[test]
    fn background_refresh_does_not_mark_visible_loading() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];

        let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

        assert!(terminal.telegram_feed.loading_channels.is_empty());
        assert_eq!(
            terminal.telegram_feed.background_loading_channels,
            vec!["marketfeed".to_string()]
        );
    }

    #[test]
    fn background_refresh_tick_skips_while_channel_fetch_in_flight() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];

        let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

        assert!(
            terminal
                .telegram_feed
                .background_loading_channels
                .is_empty()
        );
    }

    #[test]
    fn background_refresh_tick_runs_during_private_channel_scan() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.private_channel_candidates_loading = true;

        let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

        assert_eq!(
            terminal.telegram_feed.background_loading_channels,
            vec!["marketfeed".to_string()]
        );
    }

    #[test]
    fn fast_backfill_posts_do_not_alert_or_read_as_new() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.notifications_enabled = true;

        let mut live_post = sample_post("marketfeed", 11);
        live_post.source = crate::telegram_feed::TelegramFeedPostSource::FastLive;
        deliver_enabled_fast_feed_event(
            &mut terminal,
            TelegramFastFeedEvent::Loaded(
                "marketfeed".to_string(),
                Box::new(Ok(sample_page("marketfeed", vec![live_post]))),
            ),
        );
        assert!(terminal.toasts.is_empty());

        let mut backfill_post = sample_post("marketfeed", 10);
        backfill_post.source = crate::telegram_feed::TelegramFeedPostSource::FastBackfill;
        deliver_enabled_fast_feed_event(
            &mut terminal,
            TelegramFastFeedEvent::Loaded(
                "marketfeed".to_string(),
                Box::new(Ok(sample_page("marketfeed", vec![backfill_post]))),
            ),
        );

        let backfilled = terminal
            .telegram_feed
            .posts
            .iter()
            .find(|post| post.message_id == 10)
            .expect("backfilled post should be inserted");
        assert_eq!(backfilled.first_seen_ms, 0);
        assert!(terminal.toasts.is_empty());
    }

    #[test]
    fn background_refresh_is_skipped_while_fast_feed_is_connected() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_connected = true;
        terminal
            .telegram_feed
            .record_fast_connection_event(TradingTerminal::now_ms());

        let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

        assert!(terminal.telegram_feed.loading_channels.is_empty());
        assert!(
            terminal
                .telegram_feed
                .background_loading_channels
                .is_empty()
        );
    }

    #[test]
    fn private_channel_scan_requires_signed_in_fast_session() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_api_id = Some(123);

        let _task = terminal.update_telegram_feed(Message::TelegramPrivateChannelsRefresh);

        assert!(!terminal.telegram_feed.private_channel_candidates_loading);
        assert_eq!(
            terminal.telegram_feed.fast_status,
            Some(("Sign in to Telegram fast mode first".to_string(), true))
        );
    }

    #[test]
    fn loaded_private_channel_candidates_open_and_can_collapse_selector() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        assert!(!terminal.telegram_feed.private_channel_candidates_expanded);
        let request_id = terminal
            .telegram_feed
            .next_private_channel_candidates_request_id();
        terminal.telegram_feed.private_channel_candidates_loading = true;

        let _task = terminal.update_telegram_feed(Message::TelegramPrivateChannelsLoaded(
            request_id,
            Box::new(Ok(vec![
                crate::telegram_feed::TelegramPrivateChannelCandidate {
                    peer_id: 42,
                    title: "Private Macro".to_string(),
                    avatar_handle: None,
                },
            ])),
        ));

        assert!(terminal.telegram_feed.private_channel_candidates_expanded);
        assert!(!terminal.telegram_feed.private_channel_candidates_loading);
        let _task =
            terminal.update_telegram_feed(Message::ToggleTelegramPrivateChannelCandidatesExpanded);
        assert!(!terminal.telegram_feed.private_channel_candidates_expanded);
    }

    #[test]
    fn stale_private_channel_candidates_are_ignored_after_fast_mode_disable() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;
        let stale_request_id = terminal
            .telegram_feed
            .next_private_channel_candidates_request_id();
        terminal.telegram_feed.private_channel_candidates_loading = true;

        let _task = terminal.update_telegram_feed(Message::ToggleTelegramFastFeed);
        let _task = terminal.update_telegram_feed(Message::TelegramPrivateChannelsLoaded(
            stale_request_id,
            Box::new(Ok(vec![
                crate::telegram_feed::TelegramPrivateChannelCandidate {
                    peer_id: 42,
                    title: "Private Macro".to_string(),
                    avatar_handle: None,
                },
            ])),
        ));

        assert!(terminal.telegram_feed.private_channel_candidates.is_empty());
        assert!(!terminal.telegram_feed.private_channel_candidates_loading);
        assert_eq!(
            terminal.telegram_feed.fast_status,
            Some(("Fast mode disabled".to_string(), false))
        );
    }

    #[test]
    fn stale_private_channel_candidates_are_ignored_after_newer_scan() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let stale_request_id = terminal
            .telegram_feed
            .next_private_channel_candidates_request_id();
        let _current_request_id = terminal
            .telegram_feed
            .next_private_channel_candidates_request_id();
        terminal.telegram_feed.private_channel_candidates =
            vec![crate::telegram_feed::TelegramPrivateChannelCandidate {
                peer_id: 7,
                title: "Current Private".to_string(),
                avatar_handle: None,
            }];
        terminal.telegram_feed.private_channel_candidates_loading = true;
        terminal.telegram_feed.fast_status =
            Some(("Scanning Telegram channels".to_string(), false));

        let _task = terminal.update_telegram_feed(Message::TelegramPrivateChannelsLoaded(
            stale_request_id,
            Box::new(Ok(vec![
                crate::telegram_feed::TelegramPrivateChannelCandidate {
                    peer_id: 42,
                    title: "Stale Private".to_string(),
                    avatar_handle: None,
                },
            ])),
        ));

        assert_eq!(terminal.telegram_feed.private_channel_candidates.len(), 1);
        assert_eq!(
            terminal.telegram_feed.private_channel_candidates[0].title,
            "Current Private"
        );
        assert!(terminal.telegram_feed.private_channel_candidates_loading);
        assert_eq!(
            terminal.telegram_feed.fast_status,
            Some(("Scanning Telegram channels".to_string(), false))
        );
    }

    #[test]
    fn stale_fast_feed_tick_reconnects_and_falls_back_to_public_refresh() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_connected = true;
        terminal.telegram_feed.record_fast_connection_event(
            TradingTerminal::now_ms()
                .saturating_sub(crate::telegram_feed::TELEGRAM_FAST_STALE_AFTER_MS + 1),
        );
        let nonce = terminal.telegram_feed.fast_reconnect_nonce;

        let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

        assert!(!terminal.telegram_feed.fast_connected);
        assert_eq!(
            terminal.telegram_feed.fast_reconnect_nonce,
            nonce.saturating_add(1)
        );
        assert_eq!(
            terminal.telegram_feed.background_loading_channels,
            vec!["marketfeed".to_string()]
        );
        assert!(
            terminal
                .telegram_feed
                .fast_status
                .as_ref()
                .is_some_and(|(status, is_error)| status.contains("stale") && *is_error)
        );
    }

    #[test]
    fn fast_feed_toggle_is_persisted_without_disabling_public_channels() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];

        let _task = terminal.update_telegram_feed(Message::ToggleTelegramFastFeed);

        assert!(terminal.telegram_feed.fast_mode_enabled);
        assert_eq!(terminal.telegram_feed.channels, vec!["marketfeed"]);
    }

    #[test]
    fn fast_feed_disable_abandons_pending_auth_challenge() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::PasswordRequired;
        terminal.telegram_feed.fast_code_input = "12345".to_string().into();
        terminal.telegram_feed.fast_password_input = "password".to_string().into();
        terminal.telegram_feed.fast_password_hint = Some("hint".to_string());

        let _task = terminal.update_telegram_feed(Message::ToggleTelegramFastFeed);

        assert!(!terminal.telegram_feed.fast_mode_enabled);
        assert_eq!(
            terminal.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::Idle
        );
        assert!(terminal.telegram_feed.fast_code_input.is_empty());
        assert!(terminal.telegram_feed.fast_password_input.is_empty());
        assert!(terminal.telegram_feed.fast_password_hint.is_none());
    }

    #[test]
    fn stale_fast_auth_result_after_fast_mode_disable_is_ignored() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
        terminal.telegram_feed.fast_auth_in_flight = true;
        let stale_request_id = terminal.telegram_feed.next_fast_auth_request_id();

        let _task = terminal.update_telegram_feed(Message::ToggleTelegramFastFeed);
        let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
            stale_request_id,
            TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::SignedIn {
                display_name: "Alice".to_string(),
            })),
        ));

        assert_eq!(
            terminal.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::Idle
        );
        assert!(!terminal.telegram_feed.fast_connected);
        assert!(!terminal.telegram_feed.fast_auth_in_flight);
        assert_eq!(
            terminal.telegram_feed.fast_status,
            Some(("Fast mode disabled".to_string(), false))
        );
    }

    #[test]
    fn fast_feed_identity_edit_abandons_pending_auth_challenge() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
        terminal.telegram_feed.fast_auth_in_flight = true;
        terminal.telegram_feed.fast_code_input = "12345".to_string().into();
        terminal.telegram_feed.fast_password_hint = Some("hint".to_string());

        let _task =
            terminal.update_telegram_feed(Message::TelegramFastPhoneChanged("+15555550123".into()));

        assert_eq!(
            terminal.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::Idle
        );
        assert!(terminal.telegram_feed.fast_code_input.is_empty());
        assert!(terminal.telegram_feed.fast_password_hint.is_none());
        assert!(!terminal.telegram_feed.fast_auth_in_flight);
    }

    #[test]
    fn stale_fast_auth_result_after_identity_edit_is_ignored() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
        terminal.telegram_feed.fast_auth_in_flight = true;
        let stale_request_id = terminal.telegram_feed.next_fast_auth_request_id();

        let _task =
            terminal.update_telegram_feed(Message::TelegramFastPhoneChanged("+15555550123".into()));
        let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
            stale_request_id,
            TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::SignedIn {
                display_name: "Alice".to_string(),
            })),
        ));

        assert_eq!(
            terminal.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::Idle
        );
        assert!(!terminal.telegram_feed.fast_connected);
        assert!(!terminal.telegram_feed.fast_auth_in_flight);
    }

    #[test]
    fn stale_fast_code_sent_after_identity_edit_is_ignored() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
        terminal.telegram_feed.fast_auth_in_flight = true;
        let stale_request_id = terminal.telegram_feed.next_fast_auth_request_id();

        let _task =
            terminal.update_telegram_feed(Message::TelegramFastPhoneChanged("+15555550123".into()));

        let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
            stale_request_id,
            TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::CodeSent)),
        ));

        assert_eq!(
            terminal.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::Idle
        );
        assert!(!terminal.telegram_feed.fast_auth_in_flight);
        assert!(!terminal.telegram_feed.fast_connected);
    }

    #[test]
    fn fast_auth_error_status_is_sanitized() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let request_id = terminal.telegram_feed.next_fast_auth_request_id();
        terminal.telegram_feed.fast_auth_in_flight = true;

        let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
            request_id,
            TelegramFastAuthMessageResult::new(Err(
                "telegram failed api_hash=hash-secret phone_code=code-secret".to_string(),
            )),
        ));

        let status = terminal.telegram_feed.fast_status.as_ref().expect("status");
        assert_eq!(
            status,
            &("Telegram fast-mode request failed".to_string(), true)
        );
        assert!(!status.0.contains("hash-secret"));
        assert!(!status.0.contains("code-secret"));
    }

    #[test]
    fn accepted_fast_code_sent_drops_abandoned_auth_challenges() {
        let _guard = telegram_fast_pending_auth_test_lock()
            .lock()
            .expect("pending auth test lock");
        clear_telegram_fast_pending_auth();
        set_telegram_fast_pending_auth_placeholders_for_test(&[
            ("/tmp/kerosene-telegram-a.session", 1),
            ("/tmp/kerosene-telegram-a.session", 2),
            ("/tmp/kerosene-telegram-b.session", 3),
        ]);
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_auth_request_id = 2;
        terminal.telegram_feed.fast_auth_in_flight = true;

        let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
            2,
            TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::CodeSent)),
        ));

        assert_eq!(telegram_fast_pending_auth_request_ids_for_test(), vec![2]);
        clear_telegram_fast_pending_auth();
    }

    #[test]
    fn fast_code_request_is_ignored_while_auth_request_is_in_flight() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_api_id = Some(12345);
        terminal.telegram_feed.fast_api_id_input = "12345".to_string();
        terminal.telegram_feed.fast_api_hash_input = "hash".to_string().into();
        terminal.telegram_feed.fast_phone_input = "+15555550123".to_string();
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
        let request_id = terminal.telegram_feed.next_fast_auth_request_id();
        terminal.telegram_feed.fast_auth_in_flight = true;

        let _task = terminal.update_telegram_feed(Message::TelegramFastRequestCode);

        assert_eq!(terminal.telegram_feed.fast_auth_request_id, request_id);
        assert!(terminal.telegram_feed.fast_auth_in_flight);
        assert_eq!(
            terminal.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::CodeRequested
        );
    }

    #[test]
    fn fast_auth_local_session_clear_failure_does_not_mark_signed_out() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;
        terminal.telegram_feed.fast_connected = true;
        let nonce = terminal.telegram_feed.fast_reconnect_nonce;
        let request_id = terminal.telegram_feed.next_fast_auth_request_id();
        terminal.telegram_feed.fast_auth_in_flight = true;

        let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
            request_id,
            TelegramFastAuthMessageResult::new(Err(format!(
                "{TELEGRAM_FAST_SESSION_CLEAR_FAILED}: remove <config-dir>/telegram_fast.session failed"
            ))),
        ));

        assert_eq!(
            terminal.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::SignedIn
        );
        assert!(terminal.telegram_feed.fast_connected);
        assert_eq!(terminal.telegram_feed.fast_reconnect_nonce, nonce);
        let status = terminal.telegram_feed.fast_status.as_ref().expect("status");
        assert_eq!(
            status,
            &(TELEGRAM_FAST_SESSION_CLEAR_FAILED.to_string(), true)
        );
        assert!(!status.0.contains("/tmp/kerosene"));
        assert!(!terminal.telegram_feed.fast_auth_in_flight);
    }

    #[test]
    fn fast_auth_signed_out_warning_clears_local_runtime_state() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;
        terminal.telegram_feed.fast_connected = true;
        terminal
            .telegram_feed
            .record_fast_connection_event(TradingTerminal::now_ms());
        terminal.telegram_feed.fast_code_input = "12345".to_string().into();
        terminal.telegram_feed.fast_password_input = "password".to_string().into();
        terminal.telegram_feed.fast_phone_input = "+15555550123".to_string();
        let nonce = terminal.telegram_feed.fast_reconnect_nonce;
        let request_id = terminal.telegram_feed.next_fast_auth_request_id();
        terminal.telegram_feed.fast_auth_in_flight = true;

        let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
            request_id,
            TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::SignedOut {
                warning: Some(TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED.to_string()),
            })),
        ));

        assert_eq!(
            terminal.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::Idle
        );
        assert!(!terminal.telegram_feed.fast_connected);
        assert!(terminal.telegram_feed.fast_last_event_ms.is_none());
        assert!(terminal.telegram_feed.fast_code_input.is_empty());
        assert!(terminal.telegram_feed.fast_password_input.is_empty());
        assert!(terminal.telegram_feed.fast_phone_input.is_empty());
        assert_eq!(
            terminal.telegram_feed.fast_reconnect_nonce,
            nonce.saturating_add(1)
        );
        assert_eq!(
            terminal.telegram_feed.fast_status,
            Some((TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED.to_string(), true))
        );
    }

    #[test]
    fn fast_auth_signed_out_warning_status_is_sanitized() {
        assert_eq!(
            telegram_fast_signed_out_status(Some(
                "remote sign-out failed: auth_token=token-secret".to_string()
            )),
            ("Telegram fast session signed out".to_string(), false)
        );
    }

    #[test]
    fn fast_password_required_status_does_not_include_hint() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let request_id = terminal.telegram_feed.next_fast_auth_request_id();
        terminal.telegram_feed.fast_auth_in_flight = true;

        let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
            request_id,
            TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::PasswordRequired {
                hint: Some("hint-secret".to_string()),
            })),
        ));

        assert_eq!(
            terminal.telegram_feed.fast_password_hint.as_deref(),
            Some("hint-secret")
        );
        let status = terminal.telegram_feed.fast_status.as_ref().expect("status");
        assert_eq!(
            status,
            &("Telegram 2FA password required".to_string(), false)
        );
        assert!(!status.0.contains("hint-secret"));
    }

    #[test]
    fn fast_stream_status_message_is_sanitized() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());

        deliver_enabled_fast_feed_event(
            &mut terminal,
            TelegramFastFeedEvent::Status {
                connected: true,
                auth_required: false,
                message: "connected api_hash=hash-secret phone_code=code-secret".to_string(),
            },
        );

        let status = terminal.telegram_feed.fast_status.as_ref().expect("status");
        assert_eq!(status, &("Fast Telegram mode listening".to_string(), false));
        assert!(!status.0.contains("hash-secret"));
        assert!(!status.0.contains("code-secret"));
    }

    #[test]
    fn fast_feed_signed_in_result_clears_login_inputs_and_reconnects() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_api_hash_input = "hash".to_string().into();
        terminal.telegram_feed.fast_phone_input = "+15555550123".to_string();
        terminal.telegram_feed.fast_code_input = "12345".to_string().into();
        terminal.telegram_feed.fast_password_input = "password".to_string().into();
        let nonce = terminal.telegram_feed.fast_reconnect_nonce;
        let request_id = terminal.telegram_feed.next_fast_auth_request_id();
        terminal.telegram_feed.fast_auth_in_flight = true;

        let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
            request_id,
            TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::SignedIn {
                display_name: "Alice".to_string(),
            })),
        ));

        assert_eq!(
            terminal.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::SignedIn
        );
        assert!(terminal.telegram_feed.fast_connected);
        assert!(terminal.telegram_feed.fast_last_event_ms.is_some());
        assert!(terminal.telegram_feed.fast_api_hash_input.is_empty());
        assert!(terminal.telegram_feed.fast_phone_input.is_empty());
        assert!(terminal.telegram_feed.fast_code_input.is_empty());
        assert!(terminal.telegram_feed.fast_password_input.is_empty());
        assert_eq!(
            terminal.telegram_feed.fast_reconnect_nonce,
            nonce.saturating_add(1)
        );
    }

    #[test]
    fn fast_feed_status_heartbeat_keeps_connection_fresh() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_connected = true;
        terminal.telegram_feed.record_fast_connection_event(
            TradingTerminal::now_ms()
                .saturating_sub(crate::telegram_feed::TELEGRAM_FAST_STALE_AFTER_MS + 1),
        );

        deliver_enabled_fast_feed_event(
            &mut terminal,
            TelegramFastFeedEvent::Status {
                connected: true,
                auth_required: false,
                message: "Fast Telegram mode listening".to_string(),
            },
        );

        assert!(terminal.telegram_feed.fast_connected);
        assert!(
            !terminal
                .telegram_feed
                .fast_connection_stale(TradingTerminal::now_ms())
        );
    }

    #[test]
    fn stale_fast_feed_status_after_reconnect_is_ignored() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_connected = false;
        let stale_nonce = terminal.telegram_feed.fast_reconnect_nonce;
        terminal.telegram_feed.fast_reconnect_nonce = stale_nonce.saturating_add(1);

        let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
            stale_nonce,
            TelegramFastFeedEvent::Status {
                connected: true,
                auth_required: false,
                message: "Fast Telegram mode listening".to_string(),
            },
        ));

        assert!(!terminal.telegram_feed.fast_connected);
        assert_eq!(terminal.telegram_feed.fast_status, None);
    }

    #[test]
    fn adding_public_channel_invalidates_stale_fast_status() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_connected = true;
        terminal
            .telegram_feed
            .record_fast_connection_event(TradingTerminal::now_ms());
        terminal.telegram_feed.fast_status =
            Some(("Fast Telegram mode listening".to_string(), false));
        terminal.telegram_feed.channel_input = "freshfeed".to_string();
        let stale_nonce = terminal.telegram_feed.fast_reconnect_nonce;

        let _task = terminal.update_telegram_feed(Message::TelegramFeedAddChannel);

        assert_eq!(
            terminal.telegram_feed.fast_reconnect_nonce,
            stale_nonce.saturating_add(1)
        );
        assert!(!terminal.telegram_feed.fast_connected);
        assert!(terminal.telegram_feed.fast_last_event_ms.is_none());

        let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
            stale_nonce,
            TelegramFastFeedEvent::Status {
                connected: true,
                auth_required: false,
                message: "Fast Telegram mode listening".to_string(),
            },
        ));

        assert!(!terminal.telegram_feed.fast_connected);
        assert!(
            terminal
                .telegram_feed
                .fast_status
                .as_ref()
                .is_some_and(|(status, is_error)| {
                    status.contains("channel list changed") && !*is_error
                })
        );
    }

    #[test]
    fn removing_public_channel_allows_background_refresh_after_fast_status_invalidated() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string(), "otherfeed".to_string()];
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_connected = true;
        terminal
            .telegram_feed
            .record_fast_connection_event(TradingTerminal::now_ms());
        let stale_nonce = terminal.telegram_feed.fast_reconnect_nonce;

        let _task =
            terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));

        assert_eq!(
            terminal.telegram_feed.fast_reconnect_nonce,
            stale_nonce.saturating_add(1)
        );
        assert!(!terminal.telegram_feed.fast_connected);
        assert!(terminal.telegram_feed.fast_last_event_ms.is_none());

        let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

        assert_eq!(
            terminal.telegram_feed.background_loading_channels,
            vec!["otherfeed".to_string()]
        );

        let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
            stale_nonce,
            TelegramFastFeedEvent::Status {
                connected: true,
                auth_required: false,
                message: "Fast Telegram mode listening".to_string(),
            },
        ));

        assert!(!terminal.telegram_feed.fast_connected);
    }

    #[test]
    fn fast_feed_loaded_event_after_disable_is_ignored() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.fast_mode_enabled = true;
        let stale_nonce = terminal.telegram_feed.fast_reconnect_nonce;
        let _task = terminal.update_telegram_feed(Message::ToggleTelegramFastFeed);
        let mut post = sample_post("marketfeed", 22);
        post.source = crate::telegram_feed::TelegramFeedPostSource::FastLive;

        let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
            stale_nonce,
            TelegramFastFeedEvent::Loaded(
                "marketfeed".to_string(),
                Box::new(Ok(sample_page("marketfeed", vec![post]))),
            ),
        ));

        assert!(!terminal.telegram_feed.fast_mode_enabled);
        assert!(terminal.telegram_feed.posts.is_empty());
        assert_eq!(
            terminal.telegram_feed.fast_status,
            Some(("Fast mode disabled".to_string(), false))
        );
    }

    #[test]
    fn fast_feed_disconnect_status_marks_fast_feed_disconnected() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_connected = true;
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;
        let nonce = terminal.telegram_feed.fast_reconnect_nonce;

        deliver_enabled_fast_feed_event(
            &mut terminal,
            TelegramFastFeedEvent::Status {
                connected: false,
                auth_required: false,
                message: "Telegram fast feed disconnected; reconnecting".to_string(),
            },
        );

        assert!(!terminal.telegram_feed.fast_connected);
        assert_eq!(terminal.telegram_feed.fast_reconnect_nonce, nonce);
        assert_eq!(
            terminal.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::SignedIn
        );
    }

    #[test]
    fn fast_feed_request_code_is_ignored_when_signed_in() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_connected = true;
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;

        let _task = terminal.update_telegram_feed(Message::TelegramFastRequestCode);

        assert!(!terminal.telegram_feed.fast_auth_in_flight);
        assert_eq!(
            terminal.telegram_feed.fast_status,
            Some(("Fast mode is already signed in".to_string(), false))
        );
    }

    #[test]
    fn telegram_channel_list_expansion_is_runtime_only() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());

        let _task = terminal.update_telegram_feed(Message::ToggleTelegramFeedChannelsExpanded);

        assert!(terminal.telegram_feed.channels_expanded);
    }

    #[test]
    fn refreshing_existing_post_preserves_row_timing() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        let mut initial_post = sample_post("marketfeed", 1);
        initial_post.fetched_at_ms = 1_100;

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page("marketfeed", vec![initial_post])),
        );

        let mut refreshed_post = sample_post("marketfeed", 1);
        refreshed_post.text = "edited".to_string();
        refreshed_post.fetched_at_ms = 9_999;
        refreshed_post.request_duration_ms = 999;
        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page("marketfeed", vec![refreshed_post])),
        );

        assert_eq!(terminal.telegram_feed.posts.len(), 1);
        let post = &terminal.telegram_feed.posts[0];
        assert_eq!(post.text, "edited");
        assert_eq!(post.fetched_at_ms, 1_100);
        assert_eq!(post.request_duration_ms, 50);
    }

    #[test]
    fn loaded_posts_capture_ticker_mentions_with_reference_price() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.exchange_symbols = vec![exchange_symbol("BTC", "BTC")];
        terminal
            .telegram_feed
            .rebuild_ticker_mention_resolver(&terminal.exchange_symbols);
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        // The baseline is anchored to a mid recorded at/before the message time
        // (timestamp_ms 1_000), not to whenever the app noticed the post.
        terminal.record_screener_mid_samples(
            &std::collections::HashMap::from([("BTC".to_string(), 100.0)]),
            500,
        );
        let mut post = sample_post("marketfeed", 1);
        post.text = "BTC is moving".to_string();

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page("marketfeed", vec![post])),
        );

        let mentions = &terminal.telegram_feed.posts[0].ticker_mentions;
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].symbol, "BTC");
        assert_eq!(mentions[0].ticker, "BTC");
        assert_eq!(mentions[0].matched_text, "BTC");
        assert_eq!(
            mentions[0].source,
            crate::symbol_mentions::SymbolAliasSource::Ticker
        );
        assert_eq!(mentions[0].reference_price, Some(100.0));
    }

    #[test]
    fn refreshing_existing_post_preserves_ticker_reference_price() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.exchange_symbols = vec![exchange_symbol("BTC", "BTC")];
        terminal
            .telegram_feed
            .rebuild_ticker_mention_resolver(&terminal.exchange_symbols);
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        terminal.record_screener_mid_samples(
            &std::collections::HashMap::from([("BTC".to_string(), 100.0)]),
            500,
        );
        let mut initial = sample_post("marketfeed", 1);
        initial.text = "BTC is moving".to_string();

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page("marketfeed", vec![initial])),
        );
        terminal.all_mids.insert("BTC".to_string(), 105.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        let mut refreshed = sample_post("marketfeed", 1);
        refreshed.text = "edited BTC".to_string();

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page("marketfeed", vec![refreshed])),
        );

        let mentions = &terminal.telegram_feed.posts[0].ticker_mentions;
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].reference_price, Some(100.0));
    }

    #[test]
    fn reference_price_anchors_to_message_time_not_late_mid() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.exchange_symbols = vec![exchange_symbol("BTC", "BTC")];
        terminal
            .telegram_feed
            .rebuild_ticker_mention_resolver(&terminal.exchange_symbols);
        // Pre-news price recorded at/just before the message timestamp (1_000).
        terminal.record_screener_mid_samples(
            &std::collections::HashMap::from([("BTC".to_string(), 100.0)]),
            500,
        );
        // The market has already moved by the time we see the post; the current
        // (late) mid must NOT become the baseline.
        terminal.all_mids.insert("BTC".to_string(), 200.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        let mut post = sample_post("marketfeed", 1);
        post.text = "BTC is moving".to_string();

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page("marketfeed", vec![post])),
        );

        let mentions = &terminal.telegram_feed.posts[0].ticker_mentions;
        assert_eq!(mentions[0].reference_price, Some(100.0));
    }

    #[test]
    fn reference_price_suppressed_for_old_post_without_message_time_sample() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.exchange_symbols = vec![exchange_symbol("BTC", "BTC")];
        terminal
            .telegram_feed
            .rebuild_ticker_mention_resolver(&terminal.exchange_symbols);
        // A fresh mid exists, but the post is ancient (timestamp 1_000) and there
        // is no recorded mid at message time, so anchoring to it would mislead.
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        let mut post = sample_post("marketfeed", 1);
        post.text = "BTC is moving".to_string();

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page("marketfeed", vec![post])),
        );

        let mentions = &terminal.telegram_feed.posts[0].ticker_mentions;
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].reference_price, None);
        // No price captured => no baseline timestamp stamped.
        assert_eq!(mentions[0].reference_seen_ms, 0);
    }

    #[test]
    fn fill_missing_reference_uses_recent_fallback_and_is_immutable() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.exchange_symbols = vec![exchange_symbol("BTC", "BTC")];
        let mut post = sample_post("marketfeed", 1); // timestamp_ms = 1_000
        post.ticker_mentions = vec![TelegramTickerMention {
            symbol: "BTC".to_string(),
            ticker: "BTC".to_string(),
            matched_text: "BTC".to_string(),
            source: crate::symbol_mentions::SymbolAliasSource::Ticker,
            confidence: 100,
            reference_price: None,
            reference_seen_ms: 0,
        }];
        terminal.telegram_feed.posts = vec![post];

        // No message-time sample, but the post is recent relative to `now`, so the
        // current fresh mid is an acceptable baseline.
        terminal.all_mids.insert("BTC".to_string(), 100.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), 50_000);
        terminal.fill_missing_telegram_ticker_reference_prices(60_000);
        let mention = &terminal.telegram_feed.posts[0].ticker_mentions[0];
        assert_eq!(mention.reference_price, Some(100.0));
        assert_eq!(mention.reference_seen_ms, 60_000);

        // A later mids tick must NOT rebase an already-captured reference.
        terminal.all_mids.insert("BTC".to_string(), 200.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), 65_000);
        terminal.fill_missing_telegram_ticker_reference_prices(70_000);
        let mention = &terminal.telegram_feed.posts[0].ticker_mentions[0];
        assert_eq!(mention.reference_price, Some(100.0));
        assert_eq!(mention.reference_seen_ms, 60_000);
    }

    #[test]
    fn fast_profile_update_without_avatar_keeps_cached_avatar() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channel_profiles.insert(
            "marketfeed".to_string(),
            crate::telegram_feed::TelegramChannelProfile {
                channel: "marketfeed".to_string(),
                title: "@marketfeed".to_string(),
                initials: "MA".to_string(),
                avatar_url: Some("https://example.com/avatar.jpg".to_string()),
                avatar_handle: Some(ImageHandle::from_bytes(vec![0x89, b'P', b'N', b'G'])),
                avatar_loading_url: None,
                avatar_request_id: 42,
                avatar_failed_at_ms: None,
            },
        );

        let _task =
            terminal.store_telegram_channel_profile(crate::telegram_feed::TelegramChannelProfile {
                channel: "marketfeed".to_string(),
                title: "Market Feed".to_string(),
                initials: "MF".to_string(),
                avatar_url: None,
                avatar_handle: None,
                avatar_loading_url: None,
                avatar_request_id: 0,
                avatar_failed_at_ms: None,
            });

        let profile = terminal
            .telegram_feed
            .channel_profiles
            .get("marketfeed")
            .expect("profile should remain cached");
        assert_eq!(
            profile.avatar_url.as_deref(),
            Some("https://example.com/avatar.jpg")
        );
        assert!(profile.avatar_handle.is_some());
        assert_eq!(profile.title, "Market Feed");
    }

    #[tokio::test]
    async fn removing_channel_clears_cached_profile_and_fast_cursor() {
        let _cursor_guard = fast_channel_cursor_test_lock().lock().await;
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        set_fast_channel_cursor_for_test("marketfeed", 99).await;
        assert_eq!(
            fast_channel_cursor_message_id_for_test("marketfeed").await,
            99
        );
        terminal.telegram_feed.channel_profiles.insert(
            "marketfeed".to_string(),
            sample_profile("marketfeed", Some("https://example.com/avatar.jpg")),
        );

        let _task =
            terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));

        assert!(
            !terminal
                .telegram_feed
                .channels
                .contains(&"marketfeed".to_string())
        );
        assert!(
            !terminal
                .telegram_feed
                .channel_profiles
                .contains_key("marketfeed")
        );
        assert_eq!(
            fast_channel_cursor_message_id_for_test("marketfeed").await,
            0
        );
    }

    #[test]
    fn adding_private_channel_uses_scanned_candidate_and_reconnects_fast_feed() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.private_channel_candidates.push(
            crate::telegram_feed::TelegramPrivateChannelCandidate {
                peer_id: 42,
                title: "Private Macro".to_string(),
                avatar_handle: Some(ImageHandle::from_bytes(vec![0x89, b'P', b'N', b'G'])),
            },
        );
        let nonce = terminal.telegram_feed.fast_reconnect_nonce;

        let _task = terminal.update_telegram_feed(Message::TelegramFeedAddPrivateChannel(42));

        assert_eq!(terminal.telegram_feed.private_channels.len(), 1);
        assert_eq!(terminal.telegram_feed.private_channels[0].peer_id, 42);
        assert_eq!(
            terminal.telegram_feed.private_channels[0].title,
            "Private Macro"
        );
        assert_eq!(
            terminal.telegram_feed.fast_reconnect_nonce,
            nonce.saturating_add(1)
        );
        let profile = terminal
            .telegram_feed
            .channel_profiles
            .get("private:42")
            .expect("selected private channel should cache scanned profile");
        assert_eq!(profile.title, "Private Macro");
        assert!(profile.avatar_handle.is_some());
    }

    #[tokio::test]
    async fn adding_private_channel_clears_existing_fast_cursor() {
        let _cursor_guard = fast_channel_cursor_test_lock().lock().await;
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let key = crate::telegram_feed::telegram_private_channel_key(42);
        set_fast_channel_cursor_for_test(&key, 99).await;
        assert_eq!(fast_channel_cursor_message_id_for_test(&key).await, 99);
        terminal.telegram_feed.private_channel_candidates.push(
            crate::telegram_feed::TelegramPrivateChannelCandidate {
                peer_id: 42,
                title: "Private Macro".to_string(),
                avatar_handle: None,
            },
        );

        let _task = terminal.update_telegram_feed(Message::TelegramFeedAddPrivateChannel(42));

        assert_eq!(terminal.telegram_feed.private_channels.len(), 1);
        assert_eq!(fast_channel_cursor_message_id_for_test(&key).await, 0);
    }

    #[test]
    fn selected_private_channel_fast_event_is_inserted() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let key = crate::telegram_feed::telegram_private_channel_key(42);
        terminal.telegram_feed.private_channels =
            vec![crate::telegram_feed::TelegramFeedPrivateChannelConfig {
                peer_id: 42,
                title: "Private Macro".to_string(),
            }];
        let mut post = sample_post(&key, 7);
        post.source = crate::telegram_feed::TelegramFeedPostSource::FastLive;
        post.received_at_ms = 1_100;
        post.first_seen_ms = 1_100;

        deliver_enabled_fast_feed_event(
            &mut terminal,
            TelegramFastFeedEvent::Loaded(key.clone(), Box::new(Ok(sample_page(&key, vec![post])))),
        );

        assert_eq!(terminal.telegram_feed.posts.len(), 1);
        assert_eq!(terminal.telegram_feed.posts[0].channel, key);
        assert_eq!(terminal.telegram_feed.posts[0].message_id, 7);
        assert_eq!(
            terminal.telegram_feed.posts[0].source,
            crate::telegram_feed::TelegramFeedPostSource::FastLive
        );
        assert_eq!(terminal.telegram_feed.posts[0].received_at_ms, 1_100);
        assert!(terminal.telegram_feed.posts[0].applied_at_ms > 0);
    }

    #[test]
    fn fast_profile_update_with_private_avatar_keeps_incoming_handle() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let key = crate::telegram_feed::telegram_private_channel_key(42);
        terminal.telegram_feed.private_channels =
            vec![crate::telegram_feed::TelegramFeedPrivateChannelConfig {
                peer_id: 42,
                title: "Private Macro".to_string(),
            }];
        terminal
            .telegram_feed
            .channel_profiles
            .insert(key.clone(), sample_profile(&key, None));
        let mut profile = sample_profile(&key, None);
        profile.title = "Private Macro".to_string();
        profile.avatar_handle = Some(ImageHandle::from_bytes(vec![0x89, b'P', b'N', b'G']));

        let _task = terminal.store_telegram_channel_profile(profile);

        let profile = terminal
            .telegram_feed
            .channel_profiles
            .get(&key)
            .expect("private profile should be stored");
        assert_eq!(profile.title, "Private Macro");
        assert!(profile.avatar_handle.is_some());
    }

    #[test]
    fn unselected_private_channel_fast_event_is_ignored() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let key = crate::telegram_feed::telegram_private_channel_key(42);
        let page = sample_page(&key, vec![sample_post(&key, 7)]);

        deliver_enabled_fast_feed_event(
            &mut terminal,
            TelegramFastFeedEvent::Loaded(key, Box::new(Ok(page))),
        );

        assert!(terminal.telegram_feed.posts.is_empty());
    }

    #[tokio::test]
    async fn removing_private_channel_clears_posts_profile_cursor_and_reconnects() {
        let _cursor_guard = fast_channel_cursor_test_lock().lock().await;
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let key = crate::telegram_feed::telegram_private_channel_key(42);
        set_fast_channel_cursor_for_test(&key, 99).await;
        assert_eq!(fast_channel_cursor_message_id_for_test(&key).await, 99);
        terminal.telegram_feed.private_channels =
            vec![crate::telegram_feed::TelegramFeedPrivateChannelConfig {
                peer_id: 42,
                title: "Private Macro".to_string(),
            }];
        terminal.telegram_feed.posts = vec![sample_post(&key, 7)];
        terminal
            .telegram_feed
            .channel_profiles
            .insert(key.clone(), sample_profile(&key, None));
        let nonce = terminal.telegram_feed.fast_reconnect_nonce;

        let _task =
            terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel(key.clone().into()));

        assert!(terminal.telegram_feed.private_channels.is_empty());
        assert!(terminal.telegram_feed.posts.is_empty());
        assert!(!terminal.telegram_feed.channel_profiles.contains_key(&key));
        assert_eq!(
            terminal.telegram_feed.fast_reconnect_nonce,
            nonce.saturating_add(1)
        );
        assert_eq!(fast_channel_cursor_message_id_for_test(&key).await, 0);
    }

    #[test]
    fn stale_avatar_result_is_ignored() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let current_url = "https://example.com/current.jpg";
        let stale_url = "https://example.com/stale.jpg";
        let mut profile = sample_profile("marketfeed", Some(current_url));
        profile.avatar_loading_url = Some(current_url.to_string());
        profile.avatar_request_id = 2;
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal
            .telegram_feed
            .channel_profiles
            .insert("marketfeed".to_string(), profile);

        let _task = terminal.update_telegram_feed(Message::TelegramAvatarLoaded(
            "marketfeed".to_string(),
            stale_url.to_string(),
            1,
            Box::new(Ok(vec![0xFF, 0xD8, 0xFF])),
        ));

        let profile = terminal
            .telegram_feed
            .channel_profiles
            .get("marketfeed")
            .expect("profile should remain");
        assert!(profile.avatar_handle.is_none());
        assert_eq!(profile.avatar_loading_url.as_deref(), Some(current_url));
        assert_eq!(profile.avatar_request_id, 2);
    }

    #[test]
    fn avatar_failure_sets_backoff_and_suppresses_immediate_refetch() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let avatar_url = "https://example.com/avatar.jpg";
        let mut profile = sample_profile("marketfeed", Some(avatar_url));
        profile.avatar_loading_url = Some(avatar_url.to_string());
        profile.avatar_request_id = 1;
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal
            .telegram_feed
            .channel_profiles
            .insert("marketfeed".to_string(), profile);

        let _task = terminal.update_telegram_feed(Message::TelegramAvatarLoaded(
            "marketfeed".to_string(),
            avatar_url.to_string(),
            1,
            Box::new(Err("avatar failed".to_string())),
        ));

        let failed_at_ms = terminal
            .telegram_feed
            .channel_profiles
            .get("marketfeed")
            .and_then(|profile| profile.avatar_failed_at_ms)
            .expect("failure timestamp should be stored");

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page_with_avatar(
                "marketfeed",
                avatar_url,
                vec![sample_post("marketfeed", 1)],
            )),
        );

        let profile = terminal
            .telegram_feed
            .channel_profiles
            .get("marketfeed")
            .expect("profile should remain");
        assert_eq!(profile.avatar_failed_at_ms, Some(failed_at_ms));
        assert!(profile.avatar_loading_url.is_none());
        assert!(profile.avatar_handle.is_none());
    }

    #[test]
    fn adding_channel_below_public_limit_is_allowed() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = (0..TELEGRAM_FEED_MAX_PUBLIC_CHANNELS - 1)
            .map(|index| format!("channel_{index}"))
            .collect();
        terminal.telegram_feed.channel_input = "another_channel".to_string();

        let _task = terminal.update_telegram_feed(Message::TelegramFeedAddChannel);

        assert!(
            terminal
                .telegram_feed
                .channels
                .contains(&"another_channel".to_string())
        );
        assert_eq!(
            terminal.telegram_feed.channels.len(),
            TELEGRAM_FEED_MAX_PUBLIC_CHANNELS
        );
        assert!(
            terminal
                .telegram_feed
                .loading_channels
                .contains(&"another_channel".to_string())
        );
        assert_eq!(terminal.telegram_feed.last_error, None);
    }

    #[test]
    fn adding_channel_at_public_limit_is_rejected() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = (0..TELEGRAM_FEED_MAX_PUBLIC_CHANNELS)
            .map(|index| format!("channel_{index}"))
            .collect();
        terminal.telegram_feed.channel_input = "another_channel".to_string();

        let _task = terminal.update_telegram_feed(Message::TelegramFeedAddChannel);

        assert!(
            !terminal
                .telegram_feed
                .channels
                .contains(&"another_channel".to_string())
        );
        assert_eq!(
            terminal.telegram_feed.channels.len(),
            TELEGRAM_FEED_MAX_PUBLIC_CHANNELS
        );
        assert!(
            !terminal
                .telegram_feed
                .loading_channels
                .contains(&"another_channel".to_string())
        );
        assert_eq!(
            terminal.telegram_feed.last_error,
            Some(format!(
                "Telegram Feed supports up to {TELEGRAM_FEED_MAX_PUBLIC_CHANNELS} public channels"
            ))
        );
    }

    #[test]
    fn refresh_caps_overfilled_public_channel_state() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = (0..TELEGRAM_FEED_MAX_PUBLIC_CHANNELS + 4)
            .map(|index| format!("channel_{index}"))
            .collect();

        let _task = terminal.request_telegram_feed_refresh();

        assert_eq!(
            terminal.telegram_feed.channels.len(),
            TELEGRAM_FEED_MAX_PUBLIC_CHANNELS
        );
        assert_eq!(
            terminal.telegram_feed.loading_channels.len(),
            TELEGRAM_FEED_MAX_PUBLIC_CHANNELS
        );
        assert!(
            !terminal
                .telegram_feed
                .channels
                .contains(&format!("channel_{TELEGRAM_FEED_MAX_PUBLIC_CHANNELS}"))
        );
        assert_eq!(
            terminal.telegram_feed.last_error,
            Some(format!(
                "Telegram Feed supports up to {TELEGRAM_FEED_MAX_PUBLIC_CHANNELS} public channels; extra channels were ignored"
            ))
        );
    }

    #[tokio::test]
    async fn adding_public_channel_clears_existing_fast_cursor() {
        let _cursor_guard = fast_channel_cursor_test_lock().lock().await;
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        set_fast_channel_cursor_for_test("another_channel", 99).await;
        assert_eq!(
            fast_channel_cursor_message_id_for_test("another_channel").await,
            99
        );
        terminal.telegram_feed.channels.clear();
        terminal.telegram_feed.channel_input = "another_channel".to_string();

        let _task = terminal.update_telegram_feed(Message::TelegramFeedAddChannel);

        assert_eq!(
            terminal.telegram_feed.channels,
            vec!["another_channel".to_string()]
        );
        assert_eq!(
            fast_channel_cursor_message_id_for_test("another_channel").await,
            0
        );
    }

    #[test]
    fn initial_load_is_quiet_and_later_new_posts_alert() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.notifications_enabled = true;

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page(
                "marketfeed",
                vec![sample_post("marketfeed", 1)],
            )),
        );
        assert_eq!(terminal.telegram_feed.posts[0].first_seen_ms, 0);
        assert!(terminal.toasts.is_empty());

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page(
                "marketfeed",
                vec![sample_post("marketfeed", 2), sample_post("marketfeed", 1)],
            )),
        );

        let new_post = terminal
            .telegram_feed
            .posts
            .iter()
            .find(|post| post.message_id == 2)
            .expect("new post should be inserted");
        assert!(new_post.first_seen_ms > 0);
        assert_eq!(terminal.toasts.len(), 1);
        assert!(terminal.toasts[0].message.contains("@marketfeed"));
    }

    #[test]
    fn hard_refresh_does_not_alert_for_seen_post_pruned_from_rendered_feed() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.notifications_enabled = true;
        let initial_posts = (1..=(crate::telegram_feed::TELEGRAM_FEED_RENDER_LIMIT as u64 + 1))
            .map(|message_id| sample_post("marketfeed", message_id))
            .collect();

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page("marketfeed", initial_posts)),
        );

        assert!(terminal.toasts.is_empty());
        assert!(
            !terminal
                .telegram_feed
                .posts
                .iter()
                .any(|post| post.message_id == 1)
        );

        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page(
                "marketfeed",
                vec![sample_post("marketfeed", 1)],
            )),
        );

        assert!(terminal.toasts.is_empty());
    }
}
