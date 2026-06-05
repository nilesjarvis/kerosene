use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::ellipsized_text;
use crate::message::Message;
use crate::telegram_fast_feed::{
    bundled_telegram_api_hash, bundled_telegram_api_id, list_telegram_private_channel_candidates,
    request_telegram_fast_login_code, sign_out_telegram_fast, submit_telegram_fast_login_code,
    submit_telegram_fast_password,
};
use crate::telegram_feed::{
    TELEGRAM_AVATAR_RETRY_BACKOFF_MS, TelegramFastAuthOutcome, TelegramFastAuthStage,
    TelegramFastFeedEvent, TelegramFeedPage, TelegramFeedPost, TelegramTickerMention,
    fetch_telegram_avatar_bytes, fetch_telegram_channel_posts, normalize_public_channel_input,
    telegram_private_channel_peer_id_from_key,
};
use iced::Task;
use iced::widget::image::Handle as ImageHandle;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(crate) fn update_telegram_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshTelegramFeed => self.request_telegram_feed_refresh(),
            Message::TelegramFeedRefreshTick => self.request_telegram_feed_background_refresh(),
            Message::TelegramFeedLoaded(channel, result) => {
                self.handle_telegram_feed_loaded(channel, *result)
            }
            Message::TelegramAvatarLoaded(channel, avatar_url, request_id, result) => {
                self.handle_telegram_avatar_loaded(channel, avatar_url, request_id, *result);
                Task::none()
            }
            Message::ToggleTelegramFastFeed => self.toggle_telegram_fast_feed(),
            Message::TelegramFastApiIdChanged(input) => {
                self.telegram_feed.fast_api_id_input = input;
                Task::none()
            }
            Message::TelegramFastApiHashChanged(input) => {
                self.telegram_feed.fast_api_hash_input.zeroize();
                self.telegram_feed.fast_api_hash_input = input.into();
                Task::none()
            }
            Message::TelegramFastPhoneChanged(input) => {
                self.telegram_feed.fast_phone_input = input;
                Task::none()
            }
            Message::TelegramFastCodeChanged(input) => {
                self.telegram_feed.fast_code_input.zeroize();
                self.telegram_feed.fast_code_input = input.into();
                Task::none()
            }
            Message::TelegramFastPasswordChanged(input) => {
                self.telegram_feed.fast_password_input.zeroize();
                self.telegram_feed.fast_password_input = input.into();
                Task::none()
            }
            Message::TelegramFastRequestCode => self.request_telegram_fast_code(),
            Message::TelegramFastSubmitCode => self.submit_telegram_fast_code(),
            Message::TelegramFastSubmitPassword => self.submit_telegram_fast_2fa_password(),
            Message::TelegramFastSignOut => self.sign_out_telegram_fast_feed(),
            Message::TelegramFastAuthResult(result) => {
                self.handle_telegram_fast_auth_result(*result)
            }
            Message::TelegramFastFeedEvent(event) => self.handle_telegram_fast_feed_event(event),
            Message::TelegramFeedChannelInputChanged(input) => {
                self.telegram_feed.channel_input = input;
                Task::none()
            }
            Message::TelegramFeedAddChannel => self.add_telegram_feed_channel(),
            Message::TelegramPrivateChannelsRefresh => self.request_telegram_private_channels(),
            Message::TelegramPrivateChannelsLoaded(result) => {
                self.handle_telegram_private_channels_loaded(*result);
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

        self.request_telegram_feed_refresh_with_visibility(false)
    }

    fn request_telegram_feed_refresh_with_visibility(&mut self, visible: bool) -> Task<Message> {
        let channels = self.telegram_feed.channels.clone();
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
            self.telegram_feed.last_error = None;
        } else {
            self.telegram_feed.background_loading_channels = channels.clone();
        }
        Task::batch(
            channels
                .into_iter()
                .map(|channel| self.request_telegram_channel_refresh_task(channel)),
        )
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

        self.telegram_feed.channels.push(channel.clone());
        self.telegram_feed.channel_input.clear();
        self.telegram_feed.last_error = None;
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

        self.telegram_feed.private_channel_candidates_loading = true;
        self.telegram_feed.fast_status = Some(("Scanning Telegram channels".to_string(), false));
        Task::perform(list_telegram_private_channel_candidates(api_id), |result| {
            Message::TelegramPrivateChannelsLoaded(Box::new(result))
        })
    }

    fn handle_telegram_private_channels_loaded(
        &mut self,
        result: Result<Vec<crate::telegram_feed::TelegramPrivateChannelCandidate>, String>,
    ) {
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
                self.telegram_feed.fast_status = Some((err, true));
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

        self.telegram_feed
            .private_channels
            .push(candidate.to_config());
        self.telegram_feed.last_error = None;
        self.telegram_feed.fast_reconnect_nonce =
            self.telegram_feed.fast_reconnect_nonce.saturating_add(1);
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
            self.telegram_feed.channel_profiles.remove(channel);
            self.telegram_feed.fast_reconnect_nonce =
                self.telegram_feed.fast_reconnect_nonce.saturating_add(1);
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
        self.telegram_feed.channel_profiles.remove(&channel);
        self.telegram_feed.last_error = None;
        self.persist_config();
    }

    fn request_telegram_channel_refresh_task(&self, channel: String) -> Task<Message> {
        Task::perform(
            fetch_telegram_channel_posts(channel.clone()),
            move |result| Message::TelegramFeedLoaded(channel.clone(), Box::new(result)),
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
        let api_hash = self.telegram_feed.fast_api_hash_input.trim().to_string();
        let api_hash = if api_hash.is_empty() {
            bundled_telegram_api_hash().unwrap_or_default().to_string()
        } else {
            api_hash
        };
        let phone = self.telegram_feed.fast_phone_input.trim().to_string();
        self.telegram_feed.fast_auth_in_flight = true;
        self.telegram_feed.fast_status =
            Some(("Requesting Telegram login code".to_string(), false));

        Task::perform(
            request_telegram_fast_login_code(api_id, api_hash, phone),
            |result| Message::TelegramFastAuthResult(Box::new(result)),
        )
    }

    fn submit_telegram_fast_code(&mut self) -> Task<Message> {
        let Some(api_id) = self.telegram_fast_api_id() else {
            return Task::none();
        };
        let code = self.telegram_feed.fast_code_input.trim().to_string();
        self.telegram_feed.fast_auth_in_flight = true;
        self.telegram_feed.fast_status = Some(("Signing in to Telegram".to_string(), false));

        Task::perform(submit_telegram_fast_login_code(api_id, code), |result| {
            Message::TelegramFastAuthResult(Box::new(result))
        })
    }

    fn submit_telegram_fast_2fa_password(&mut self) -> Task<Message> {
        let Some(api_id) = self.telegram_fast_api_id() else {
            return Task::none();
        };
        let password = self.telegram_feed.fast_password_input.trim().to_string();
        self.telegram_feed.fast_auth_in_flight = true;
        self.telegram_feed.fast_status =
            Some(("Checking Telegram 2FA password".to_string(), false));

        Task::perform(submit_telegram_fast_password(api_id, password), |result| {
            Message::TelegramFastAuthResult(Box::new(result))
        })
    }

    fn sign_out_telegram_fast_feed(&mut self) -> Task<Message> {
        let Some(api_id) = self.telegram_fast_api_id() else {
            return Task::none();
        };
        self.telegram_feed.fast_auth_in_flight = true;
        self.telegram_feed.fast_status = Some(("Signing out of Telegram".to_string(), false));

        Task::perform(sign_out_telegram_fast(api_id), |result| {
            Message::TelegramFastAuthResult(Box::new(result))
        })
    }

    fn handle_telegram_fast_auth_result(
        &mut self,
        result: Result<TelegramFastAuthOutcome, String>,
    ) -> Task<Message> {
        self.telegram_feed.fast_auth_in_flight = false;
        match result {
            Ok(TelegramFastAuthOutcome::CodeSent) => {
                self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
                self.telegram_feed.fast_status = Some(("Telegram code sent".to_string(), false));
            }
            Ok(TelegramFastAuthOutcome::PasswordRequired { hint }) => {
                self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::PasswordRequired;
                self.telegram_feed.fast_password_hint = hint.clone();
                self.telegram_feed.fast_status = Some((
                    hint.map(|hint| format!("Telegram 2FA password required; hint: {hint}"))
                        .unwrap_or_else(|| "Telegram 2FA password required".to_string()),
                    false,
                ));
            }
            Ok(TelegramFastAuthOutcome::SignedIn { display_name }) => {
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
            Ok(TelegramFastAuthOutcome::SignedOut) => {
                self.telegram_feed.fast_auth_stage = TelegramFastAuthStage::Idle;
                self.telegram_feed.fast_connected = false;
                self.telegram_feed.clear_fast_connection_event();
                self.telegram_feed.fast_code_input.zeroize();
                self.telegram_feed.fast_password_input.zeroize();
                self.telegram_feed.fast_phone_input.clear();
                self.telegram_feed.fast_reconnect_nonce =
                    self.telegram_feed.fast_reconnect_nonce.saturating_add(1);
                self.telegram_feed.fast_status =
                    Some(("Telegram fast session signed out".to_string(), false));
            }
            Err(err) => {
                self.telegram_feed.fast_status = Some((err, true));
            }
        }
        Task::none()
    }

    fn handle_telegram_fast_feed_event(&mut self, event: TelegramFastFeedEvent) -> Task<Message> {
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
                }
                self.telegram_feed.fast_status = Some((message, auth_required));
                Task::none()
            }
            TelegramFastFeedEvent::Loaded(channel, result) => {
                self.telegram_feed
                    .record_fast_connection_event(Self::now_ms());
                self.handle_telegram_feed_loaded(channel, *result)
            }
        }
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
                            now_ms,
                            &previous_mentions,
                        );
                        let existing_post = &mut self.telegram_feed.posts[existing_index];
                        existing_post.text = post.text;
                        existing_post.timestamp_ms = post.timestamp_ms;
                        existing_post.url = post.url;
                        existing_post.ticker_mentions = mentions;
                    } else {
                        if had_seen_posts && !already_seen {
                            post.first_seen_ms = now_ms;
                        }
                        let mentions =
                            self.telegram_ticker_mentions_for_text(&post.text, now_ms, &[]);
                        post.ticker_mentions = mentions;
                        if had_seen_posts && !already_seen {
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
                    self.telegram_feed.last_error = Some(err);
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
            post.ticker_mentions =
                self.telegram_ticker_mentions_for_text(&post.text, now_ms, &previous_mentions);
        }
        self.telegram_feed.posts = posts;
    }

    pub(crate) fn fill_missing_telegram_ticker_reference_prices(&mut self, now_ms: u64) {
        if self.telegram_feed.posts.is_empty() {
            return;
        }

        let mut posts = std::mem::take(&mut self.telegram_feed.posts);
        for post in &mut posts {
            for mention in &mut post.ticker_mentions {
                if mention.reference_price.is_none() {
                    mention.reference_price = self.resolve_mid_for_symbol(&mention.symbol);
                    if mention.reference_price.is_some() {
                        mention.reference_seen_ms = now_ms;
                    }
                }
            }
        }
        self.telegram_feed.posts = posts;
    }

    fn telegram_ticker_mentions_for_text(
        &self,
        text: &str,
        reference_seen_ms: u64,
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
                    if mention.reference_price.is_none() {
                        mention.reference_price = self.resolve_mid_for_symbol(&matched.symbol_key);
                        if mention.reference_price.is_some() {
                            mention.reference_seen_ms = reference_seen_ms;
                        }
                    }
                    mention
                } else {
                    TelegramTickerMention {
                        reference_price: self.resolve_mid_for_symbol(&matched.symbol_key),
                        reference_seen_ms,
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
            profile.avatar_handle = existing.avatar_handle.clone();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType};
    use crate::config::KeroseneConfig;

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

        terminal.telegram_feed.channels.clear();
        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page(
                "marketfeed",
                vec![sample_post("marketfeed", 1)],
            ))),
        ));

        assert!(terminal.telegram_feed.posts.is_empty());
        assert!(terminal.telegram_feed.loading_channels.is_empty());
        assert!(
            terminal
                .telegram_feed
                .background_loading_channels
                .is_empty()
        );
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

        let _task =
            terminal.update_telegram_feed(Message::TelegramPrivateChannelsLoaded(Box::new(Ok(
                vec![crate::telegram_feed::TelegramPrivateChannelCandidate {
                    peer_id: 42,
                    title: "Private Macro".to_string(),
                    avatar_handle: None,
                }],
            ))));

        assert!(terminal.telegram_feed.private_channel_candidates_expanded);
        let _task =
            terminal.update_telegram_feed(Message::ToggleTelegramPrivateChannelCandidatesExpanded);
        assert!(!terminal.telegram_feed.private_channel_candidates_expanded);
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
    fn fast_feed_signed_in_result_clears_login_inputs_and_reconnects() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_api_hash_input = "hash".to_string().into();
        terminal.telegram_feed.fast_phone_input = "+15555550123".to_string();
        terminal.telegram_feed.fast_code_input = "12345".to_string().into();
        terminal.telegram_feed.fast_password_input = "password".to_string().into();
        let nonce = terminal.telegram_feed.fast_reconnect_nonce;

        let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(Box::new(Ok(
            TelegramFastAuthOutcome::SignedIn {
                display_name: "Alice".to_string(),
            },
        ))));

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

        let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
            TelegramFastFeedEvent::Status {
                connected: true,
                auth_required: false,
                message: "Fast Telegram mode listening".to_string(),
            },
        ));

        assert!(terminal.telegram_feed.fast_connected);
        assert!(
            !terminal
                .telegram_feed
                .fast_connection_stale(TradingTerminal::now_ms())
        );
    }

    #[test]
    fn fast_feed_disconnect_status_marks_fast_feed_disconnected() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_connected = true;
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;
        let nonce = terminal.telegram_feed.fast_reconnect_nonce;

        let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
            TelegramFastFeedEvent::Status {
                connected: false,
                auth_required: false,
                message: "Telegram fast feed disconnected; reconnecting".to_string(),
            },
        ));

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

        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page("marketfeed", vec![initial_post]))),
        ));

        let mut refreshed_post = sample_post("marketfeed", 1);
        refreshed_post.text = "edited".to_string();
        refreshed_post.fetched_at_ms = 9_999;
        refreshed_post.request_duration_ms = 999;
        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page("marketfeed", vec![refreshed_post]))),
        ));

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
        let mut post = sample_post("marketfeed", 1);
        post.text = "BTC is moving".to_string();

        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page("marketfeed", vec![post]))),
        ));

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
        let mut initial = sample_post("marketfeed", 1);
        initial.text = "BTC is moving".to_string();

        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page("marketfeed", vec![initial]))),
        ));
        terminal.all_mids.insert("BTC".to_string(), 105.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        let mut refreshed = sample_post("marketfeed", 1);
        refreshed.text = "edited BTC".to_string();

        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page("marketfeed", vec![refreshed]))),
        ));

        let mentions = &terminal.telegram_feed.posts[0].ticker_mentions;
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].reference_price, Some(100.0));
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

    #[test]
    fn removing_channel_clears_cached_profile() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.channel_profiles.insert(
            "marketfeed".to_string(),
            sample_profile("marketfeed", Some("https://example.com/avatar.jpg")),
        );

        let _task = terminal
            .update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".to_string()));

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
    }

    #[test]
    fn adding_private_channel_uses_scanned_candidate_and_reconnects_fast_feed() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.private_channel_candidates.push(
            crate::telegram_feed::TelegramPrivateChannelCandidate {
                peer_id: 42,
                title: "Private Macro".to_string(),
                avatar_handle: None,
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

        let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
            TelegramFastFeedEvent::Loaded(
                key.clone(),
                Box::new(Ok(sample_page(&key, vec![sample_post(&key, 7)]))),
            ),
        ));

        assert_eq!(terminal.telegram_feed.posts.len(), 1);
        assert_eq!(terminal.telegram_feed.posts[0].channel, key);
        assert_eq!(terminal.telegram_feed.posts[0].message_id, 7);
    }

    #[test]
    fn unselected_private_channel_fast_event_is_ignored() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let key = crate::telegram_feed::telegram_private_channel_key(42);
        let page = sample_page(&key, vec![sample_post(&key, 7)]);

        let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
            TelegramFastFeedEvent::Loaded(key, Box::new(Ok(page))),
        ));

        assert!(terminal.telegram_feed.posts.is_empty());
    }

    #[test]
    fn removing_private_channel_clears_posts_profile_and_reconnects() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let key = crate::telegram_feed::telegram_private_channel_key(42);
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

        let _task = terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel(key.clone()));

        assert!(terminal.telegram_feed.private_channels.is_empty());
        assert!(terminal.telegram_feed.posts.is_empty());
        assert!(!terminal.telegram_feed.channel_profiles.contains_key(&key));
        assert_eq!(
            terminal.telegram_feed.fast_reconnect_nonce,
            nonce.saturating_add(1)
        );
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

        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page_with_avatar(
                "marketfeed",
                avatar_url,
                vec![sample_post("marketfeed", 1)],
            ))),
        ));

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
    fn adding_channel_after_many_existing_channels_is_allowed() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = (0..16).map(|index| format!("channel_{index}")).collect();
        terminal.telegram_feed.channel_input = "another_channel".to_string();

        let _task = terminal.update_telegram_feed(Message::TelegramFeedAddChannel);

        assert!(
            terminal
                .telegram_feed
                .channels
                .contains(&"another_channel".to_string())
        );
        assert_eq!(terminal.telegram_feed.channels.len(), 17);
        assert_eq!(terminal.telegram_feed.last_error, None);
    }

    #[test]
    fn initial_load_is_quiet_and_later_new_posts_alert() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.notifications_enabled = true;

        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page(
                "marketfeed",
                vec![sample_post("marketfeed", 1)],
            ))),
        ));
        assert_eq!(terminal.telegram_feed.posts[0].first_seen_ms, 0);
        assert!(terminal.toasts.is_empty());

        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page(
                "marketfeed",
                vec![sample_post("marketfeed", 2), sample_post("marketfeed", 1)],
            ))),
        ));

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

        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page("marketfeed", initial_posts))),
        ));

        assert!(terminal.toasts.is_empty());
        assert!(
            !terminal
                .telegram_feed
                .posts
                .iter()
                .any(|post| post.message_id == 1)
        );

        let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page(
                "marketfeed",
                vec![sample_post("marketfeed", 1)],
            ))),
        ));

        assert!(terminal.toasts.is_empty());
    }
}
