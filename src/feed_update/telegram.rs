use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::{ellipsized_text, redact_sensitive_response_text};
use crate::message::Message;
use crate::telegram_fast_feed::list_telegram_private_channel_candidates;
use crate::telegram_feed::{
    TELEGRAM_AVATAR_RETRY_BACKOFF_MS, TELEGRAM_FEED_MAX_PUBLIC_CHANNELS,
    TELEGRAM_MEDIA_RETRY_BACKOFF_MS, TelegramFeedPage, TelegramFeedPost, TelegramFeedPostSource,
    TelegramPostMedia, TelegramTickerMention, fetch_telegram_avatar_bytes,
    fetch_telegram_channel_posts, fetch_telegram_media_bytes, normalize_public_channel_input,
    normalized_channel_list, telegram_private_channel_peer_id_from_key,
};
use iced::Task;
use iced::widget::image::Handle as ImageHandle;
use zeroize::{Zeroize, Zeroizing};

mod fast;

/// When no recorded mid exists at a post's publication time, the current live mid
/// is only an honest baseline if the post is this recent; older posts get no
/// price-impact percentage rather than one anchored to a late price.
const TELEGRAM_REFERENCE_FALLBACK_MAX_AGE_MS: u64 = 90_000;
const TELEGRAM_MAX_ALERTS_PER_REFRESH: usize = 3;

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
            Message::TelegramMediaLoaded(channel, message_id, media_url, request_id, result) => {
                self.handle_telegram_media_loaded(
                    channel, message_id, media_url, request_id, *result,
                );
                Task::none()
            }
            Message::ToggleTelegramFastFeed => self.toggle_telegram_fast_feed(),
            Message::TelegramFeedDismissOnboarding => {
                self.telegram_feed.onboarding_dismissed = true;
                self.persist_config();
                Task::none()
            }
            Message::TelegramFeedShowOnboarding => self.show_telegram_onboarding(),
            Message::ToggleTelegramFastAdvanced => {
                self.telegram_feed.fast_advanced_expanded =
                    !self.telegram_feed.fast_advanced_expanded;
                Task::none()
            }
            Message::TelegramFastCountryCodeChanged(code) => {
                self.clear_abandoned_telegram_fast_auth_challenge();
                self.telegram_feed.fast_country_code = code;
                Task::none()
            }
            Message::TelegramFastEditNumber => {
                // Return from the code step to the phone step, discarding the
                // pending code challenge.
                self.clear_abandoned_telegram_fast_auth_challenge();
                self.telegram_feed.fast_code_sent_at_ms = None;
                Task::none()
            }
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
                // Telegram login codes are 5 digits; keep only those so the
                // digit-cell display stays aligned regardless of paste/format.
                let raw = input.into_zeroizing();
                let mut digits = Zeroizing::new(String::new());
                for ch in raw.chars().filter(char::is_ascii_digit).take(5) {
                    digits.push(ch);
                }
                self.telegram_feed.fast_code_input = digits.into();
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
            .map(|channel| self.request_telegram_channel_refresh_task(channel));
        Task::batch(tasks)
    }

    fn add_telegram_feed_channel(&mut self) -> Task<Message> {
        let channel = match normalize_public_channel_input(&self.telegram_feed.channel_input) {
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
        if !self.telegram_feed.signed_in() {
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
                let mut alert_messages = Vec::new();
                let mut new_post_count = 0;
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
                        let mentions = self.telegram_ticker_mentions_for_text(
                            &post.text,
                            post.timestamp_ms,
                            now_ms,
                            &self.telegram_feed.posts[existing_index].ticker_mentions,
                        );
                        let existing_post = &mut self.telegram_feed.posts[existing_index];
                        existing_post.text = post.text;
                        existing_post.timestamp_ms = post.timestamp_ms;
                        existing_post.url = post.url;
                        existing_post.ticker_mentions = mentions;
                        existing_post.applied_at_ms = post.applied_at_ms;
                        existing_post.media =
                            merge_telegram_post_media(existing_post.media.take(), post.media);
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
                        if treat_as_new && self.telegram_feed.notifications_enabled {
                            new_post_count += 1;
                            if alert_messages.len() < TELEGRAM_MAX_ALERTS_PER_REFRESH {
                                alert_messages.push(telegram_post_alert_message(&post));
                            }
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
                for message in alert_messages {
                    self.push_telegram_feed_alert(message);
                }
                if new_post_count > TELEGRAM_MAX_ALERTS_PER_REFRESH {
                    self.push_telegram_feed_alert(format!(
                        "{} more Telegram messages",
                        new_post_count - TELEGRAM_MAX_ALERTS_PER_REFRESH
                    ));
                }
                let media_task = self.schedule_telegram_media_fetches(&channel);
                Task::batch([avatar_task, media_task])
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
            post.ticker_mentions = self.telegram_ticker_mentions_for_text(
                &post.text,
                post.timestamp_ms,
                now_ms,
                &post.ticker_mentions,
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
                let previous = previous_mentions
                    .iter()
                    .find(|mention| mention.symbol == matched.symbol_key);
                let mut reference_price = previous.and_then(|mention| mention.reference_price);
                let mut reference_seen_ms = previous.map_or(0, |mention| mention.reference_seen_ms);
                if reference_price.is_none()
                    && let Some(price) = self.telegram_reference_price_for_mention(
                        &matched.symbol_key,
                        post_timestamp_ms,
                        now_ms,
                    )
                {
                    reference_price = Some(price);
                    reference_seen_ms = now_ms;
                }
                TelegramTickerMention {
                    reference_seen_ms,
                    reference_price,
                    symbol: matched.symbol_key,
                    ticker: matched.ticker,
                    matched_text: matched.matched_text,
                    source: matched.source,
                    confidence: matched.confidence,
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
        if let Some(existing) = self.telegram_feed.channel_profiles.get(&profile.channel) {
            if profile.avatar_url.is_none() {
                profile.avatar_url = existing.avatar_url.clone();
            }
            if existing.avatar_url == profile.avatar_url {
                if profile.avatar_handle.is_none() {
                    profile.avatar_handle = existing.avatar_handle.clone();
                }
                profile.avatar_loading_url = existing.avatar_loading_url.clone();
                profile.avatar_request_id = existing.avatar_request_id;
                profile.avatar_failed_at_ms = existing.avatar_failed_at_ms;
            }
        }

        let task = if let Some(avatar_url) = &profile.avatar_url
            && profile.avatar_handle.is_none()
            && profile.avatar_loading_url.as_deref() != Some(avatar_url.as_str())
            && !profile.avatar_failed_at_ms.is_some_and(|failed_at_ms| {
                now_ms.saturating_sub(failed_at_ms) < TELEGRAM_AVATAR_RETRY_BACKOFF_MS
            }) {
            self.telegram_feed.next_avatar_request_id =
                self.telegram_feed.next_avatar_request_id.saturating_add(1);
            profile.avatar_loading_url = Some(avatar_url.clone());
            profile.avatar_request_id = self.telegram_feed.next_avatar_request_id;
            profile.avatar_failed_at_ms = None;

            let channel = profile.channel.clone();
            let avatar_url = avatar_url.clone();
            let request_id = profile.avatar_request_id;
            Task::perform(
                fetch_telegram_avatar_bytes(channel.clone(), avatar_url.clone()),
                move |result| {
                    Message::TelegramAvatarLoaded(
                        channel.clone(),
                        avatar_url.clone(),
                        request_id,
                        Box::new(result),
                    )
                },
            )
        } else {
            Task::none()
        };
        self.telegram_feed
            .channel_profiles
            .insert(profile.channel.clone(), profile);
        task
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

    fn schedule_telegram_media_fetches(&mut self, channel: &str) -> Task<Message> {
        let now_ms = Self::now_ms();
        // First pass identifies public posts whose preview still needs fetching;
        // fast-mode media carries no URL and is downloaded by the stream instead.
        let targets = self
            .telegram_feed
            .posts
            .iter()
            .filter(|post| post.channel == channel)
            .filter_map(|post| {
                let media = post.media.as_ref()?;
                let url = media.url.as_ref()?;
                let needs_fetch = media.handle.is_none()
                    && media.loading_url.as_deref() != Some(url.as_str())
                    && !media.failed_at_ms.is_some_and(|failed_at_ms| {
                        now_ms.saturating_sub(failed_at_ms) < TELEGRAM_MEDIA_RETRY_BACKOFF_MS
                    });
                needs_fetch.then(|| (post.message_id, url.clone()))
            })
            .collect::<Vec<_>>();
        if targets.is_empty() {
            return Task::none();
        }

        let mut tasks = Vec::with_capacity(targets.len());
        for (message_id, url) in targets {
            self.telegram_feed.next_media_request_id =
                self.telegram_feed.next_media_request_id.saturating_add(1);
            let request_id = self.telegram_feed.next_media_request_id;
            if let Some(media) = self
                .telegram_feed
                .posts
                .iter_mut()
                .find(|post| post.channel == channel && post.message_id == message_id)
                .and_then(|post| post.media.as_mut())
            {
                media.loading_url = Some(url.clone());
                media.request_id = request_id;
                media.failed_at_ms = None;
            }
            let channel = channel.to_string();
            tasks.push(Task::perform(
                fetch_telegram_media_bytes(channel.clone(), message_id, url.clone()),
                move |result| {
                    Message::TelegramMediaLoaded(
                        channel.clone(),
                        message_id,
                        url.clone(),
                        request_id,
                        Box::new(result),
                    )
                },
            ));
        }
        Task::batch(tasks)
    }

    fn handle_telegram_media_loaded(
        &mut self,
        channel: String,
        message_id: u64,
        media_url: String,
        request_id: u64,
        result: Result<Vec<u8>, String>,
    ) {
        if !self.telegram_feed.feed_source_selected(&channel) {
            return;
        }
        let Some(media) = self
            .telegram_feed
            .posts
            .iter_mut()
            .find(|post| post.channel == channel && post.message_id == message_id)
            .and_then(|post| post.media.as_mut())
        else {
            return;
        };
        // Guard against a stale response landing on media that has since changed,
        // been superseded by a newer fetch, or already resolved.
        if media.url.as_deref() != Some(media_url.as_str()) {
            return;
        }
        if media.request_id != request_id {
            return;
        }
        if media.loading_url.as_deref() == Some(media_url.as_str()) {
            media.loading_url = None;
        } else {
            return;
        }

        match result {
            Ok(bytes) => {
                media.handle = Some(ImageHandle::from_bytes(bytes));
                media.request_id = 0;
                media.failed_at_ms = None;
            }
            Err(_) => {
                media.handle = None;
                media.request_id = 0;
                media.failed_at_ms = Some(Self::now_ms());
            }
        }
    }
}

/// Reconciles a post's media across a refresh or a follow-up download. A freshly
/// downloaded handle always wins (e.g. a fast-mode preview arriving late);
/// otherwise an unchanged media reference keeps whatever was already downloaded
/// or is mid-fetch, so a refresh never discards a loaded preview or restarts its
/// fetch. A changed reference is replaced and re-fetched.
fn merge_telegram_post_media(
    existing: Option<TelegramPostMedia>,
    incoming: Option<TelegramPostMedia>,
) -> Option<TelegramPostMedia> {
    match (existing, incoming) {
        (existing, None) => existing,
        (None, incoming) => incoming,
        (Some(existing), Some(incoming)) => {
            if incoming.handle.is_some() {
                // A freshly decoded preview always wins (e.g. a fast follow-up).
                Some(incoming)
            } else if existing.handle.is_some()
                && (existing.url.is_none() || existing.url == incoming.url)
            {
                // Keep an already-loaded preview across a refresh: fast-sourced
                // media (url == None) has no public URL to re-fetch, and an
                // unchanged public reference points at the same image. This stops
                // a public refresh from discarding a loaded handle and re-fetching.
                Some(existing)
            } else if incoming.failed_at_ms.is_some() {
                // Surface a download failure onto the pending placeholder so the
                // card can stop showing an indefinite loading state.
                Some(incoming)
            } else if existing.url == incoming.url {
                // Same pending reference: keep in-flight / backoff bookkeeping.
                Some(existing)
            } else {
                // A genuinely changed reference is replaced and re-fetched.
                Some(incoming)
            }
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

#[cfg(test)]
mod tests;
