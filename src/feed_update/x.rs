use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::ellipsized_text;
use crate::message::Message;
use crate::x_feed::{
    X_FEED_MAX_SOURCES, XFeedPage, XFeedPost, XFeedStreamEvent, XTickerMention,
    fetch_x_recent_posts, normalize_x_bearer_token_input, normalize_x_handle_input,
};
use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(crate) fn update_x_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshXFeed => self.request_x_feed_refresh(),
            Message::XFeedRefreshTick => self.request_x_feed_background_refresh(),
            Message::XFeedLoaded(result) => self.handle_x_feed_loaded(*result, true),
            Message::XFeedStreamEvent(event) => self.handle_x_feed_stream_event(event),
            Message::XFeedBearerTokenChanged(input) => {
                self.x_feed.bearer_token_input.zeroize();
                self.x_feed.bearer_token_input = input.into();
                Task::none()
            }
            Message::SaveXFeedBearerToken => self.save_x_feed_bearer_token(),
            Message::XFeedSourceInputChanged(input) => {
                self.x_feed.source_input = input;
                Task::none()
            }
            Message::XFeedAddSource => self.add_x_feed_source(),
            Message::XFeedRemoveSource(handle) => {
                self.remove_x_feed_source(&handle);
                Task::none()
            }
            Message::ToggleXFeedStreaming => {
                self.x_feed.streaming_enabled = !self.x_feed.streaming_enabled;
                self.x_feed.stream_connected = false;
                self.x_feed.stream_reconnect_nonce =
                    self.x_feed.stream_reconnect_nonce.saturating_add(1);
                self.x_feed.stream_status = Some((
                    if self.x_feed.streaming_enabled {
                        "X stream enabled".to_string()
                    } else {
                        "X stream disabled".to_string()
                    },
                    false,
                ));
                self.persist_config();
                Task::none()
            }
            Message::ToggleXFeedNotifications => {
                self.x_feed.notifications_enabled = !self.x_feed.notifications_enabled;
                self.persist_config();
                Task::none()
            }
            Message::ToggleXFeedSourcesExpanded => {
                self.x_feed.sources_expanded = !self.x_feed.sources_expanded;
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn request_x_feed_refresh(&mut self) -> Task<Message> {
        self.request_x_feed_refresh_with_visibility(true)
    }

    pub(crate) fn request_x_feed_background_refresh(&mut self) -> Task<Message> {
        if self.x_feed.streaming_enabled && self.x_feed.stream_connected {
            return Task::none();
        }
        self.request_x_feed_refresh_with_visibility(false)
    }

    fn request_x_feed_refresh_with_visibility(&mut self, visible: bool) -> Task<Message> {
        if self.x_feed.bearer_token.trim().is_empty() {
            self.x_feed.last_error = Some("Enter an X API bearer token".to_string());
            return Task::none();
        }
        if self.x_feed.handles.is_empty() {
            self.x_feed.last_error = Some("Add a public X handle".to_string());
            return Task::none();
        }

        if visible {
            self.x_feed.loading = true;
            self.x_feed.last_error = None;
        } else {
            self.x_feed.background_loading = true;
        }

        self.request_x_feed_refresh_task()
    }

    fn request_x_feed_refresh_task(&self) -> Task<Message> {
        let token = self.x_feed.bearer_token.trim().to_string();
        let handles = self.x_feed.handles.clone();
        Task::perform(fetch_x_recent_posts(token, handles), |result| {
            Message::XFeedLoaded(Box::new(result))
        })
    }

    fn save_x_feed_bearer_token(&mut self) -> Task<Message> {
        self.x_feed.bearer_token.zeroize();
        self.x_feed.bearer_token =
            normalize_x_bearer_token_input(&self.x_feed.bearer_token_input).into();
        self.x_feed.bearer_token_input.zeroize();
        self.x_feed.bearer_token_input = self.x_feed.bearer_token.clone();
        self.x_feed.stream_connected = false;
        self.x_feed.stream_reconnect_nonce = self.x_feed.stream_reconnect_nonce.saturating_add(1);
        self.x_feed.stream_status = if self.x_feed.bearer_token.trim().is_empty() {
            Some(("X bearer token cleared".to_string(), false))
        } else {
            Some(("X bearer token saved".to_string(), false))
        };
        self.persist_x_secret();
        self.persist_config();
        if !self.x_feed.bearer_token.trim().is_empty() && self.x_feed.posts.is_empty() {
            return self.request_x_feed_refresh();
        }
        Task::none()
    }

    fn add_x_feed_source(&mut self) -> Task<Message> {
        let input = self.x_feed.source_input.clone();
        let handle = match normalize_x_handle_input(&input) {
            Ok(handle) => handle,
            Err(err) => {
                self.x_feed.last_error = Some(err);
                return Task::none();
            }
        };

        if self
            .x_feed
            .handles
            .iter()
            .any(|existing| existing == &handle)
        {
            self.x_feed.source_input.clear();
            self.x_feed.last_error = Some(format!("@{handle} is already in the feed"));
            return Task::none();
        }

        if self.x_feed.handles.len() >= X_FEED_MAX_SOURCES {
            self.x_feed.last_error = Some(format!(
                "X Feed supports up to {X_FEED_MAX_SOURCES} sources"
            ));
            return Task::none();
        }

        self.x_feed.handles.push(handle);
        self.x_feed.source_input.clear();
        self.x_feed.last_error = None;
        self.x_feed.stream_connected = false;
        self.x_feed.stream_reconnect_nonce = self.x_feed.stream_reconnect_nonce.saturating_add(1);
        self.persist_config();
        if !self.x_feed.bearer_token.trim().is_empty() {
            return self.request_x_feed_refresh();
        }
        Task::none()
    }

    fn remove_x_feed_source(&mut self, handle: &str) {
        let Ok(handle) = normalize_x_handle_input(handle) else {
            return;
        };
        self.x_feed.handles.retain(|existing| existing != &handle);
        self.x_feed.posts.retain(|post| post.username != handle);
        self.x_feed.clear_seen_posts();
        self.x_feed.last_error = None;
        self.x_feed.stream_connected = false;
        self.x_feed.stream_reconnect_nonce = self.x_feed.stream_reconnect_nonce.saturating_add(1);
        self.persist_config();
    }

    fn handle_x_feed_stream_event(&mut self, event: XFeedStreamEvent) -> Task<Message> {
        match event {
            XFeedStreamEvent::Status { connected, message } => {
                self.x_feed.stream_connected = connected;
                self.x_feed.stream_status = Some((message, !connected));
                Task::none()
            }
            XFeedStreamEvent::Loaded(result) => self.handle_x_feed_loaded(*result, false),
        }
    }

    fn handle_x_feed_loaded(
        &mut self,
        result: Result<XFeedPage, String>,
        visible_result: bool,
    ) -> Task<Message> {
        let was_visible_loading = self.x_feed.loading || visible_result;
        self.x_feed.loading = false;
        self.x_feed.background_loading = false;

        match result {
            Ok(page) => {
                let now_ms = Self::now_ms();
                self.x_feed.last_error = None;
                for (id, profile) in page.profiles {
                    self.x_feed.profiles.insert(id, profile);
                }

                let had_seen_posts = self.x_feed.has_seen_posts();
                let mut new_posts = Vec::new();
                for mut post in page.posts {
                    let already_seen = self.x_feed.record_seen_post(&post.id);
                    if let Some(existing_index) = self
                        .x_feed
                        .posts
                        .iter()
                        .position(|existing| existing.id == post.id)
                    {
                        let previous_mentions =
                            self.x_feed.posts[existing_index].ticker_mentions.clone();
                        let mentions =
                            self.x_ticker_mentions_for_text(&post.text, now_ms, &previous_mentions);
                        let existing_post = &mut self.x_feed.posts[existing_index];
                        existing_post.text = post.text;
                        existing_post.timestamp_ms = post.timestamp_ms;
                        existing_post.url = post.url;
                        existing_post.username = post.username;
                        existing_post.author_id = post.author_id;
                        existing_post.ticker_mentions = mentions;
                    } else {
                        if had_seen_posts && !already_seen {
                            post.first_seen_ms = now_ms;
                        }
                        post.ticker_mentions =
                            self.x_ticker_mentions_for_text(&post.text, now_ms, &[]);
                        if had_seen_posts && !already_seen {
                            new_posts.push(post.clone());
                        }
                        self.x_feed.posts.push(post);
                    }
                }

                self.x_feed.posts.sort_by(|left, right| {
                    right
                        .timestamp_ms
                        .cmp(&left.timestamp_ms)
                        .then_with(|| right.id.cmp(&left.id))
                });
                self.x_feed
                    .posts
                    .dedup_by(|left, right| left.id == right.id);
                self.x_feed
                    .posts
                    .truncate(crate::x_feed::X_FEED_RENDER_LIMIT);
                self.x_feed.last_refresh_ms = Some(now_ms);
                if self.x_feed.notifications_enabled {
                    self.push_new_x_post_alerts(&new_posts);
                }
                Task::none()
            }
            Err(err) => {
                if was_visible_loading || self.x_feed.posts.is_empty() {
                    self.x_feed.last_error = Some(err);
                }
                Task::none()
            }
        }
    }

    pub(crate) fn refresh_x_ticker_mentions(&mut self) {
        if self.x_feed.posts.is_empty() {
            return;
        }

        let now_ms = Self::now_ms();
        let mut posts = std::mem::take(&mut self.x_feed.posts);
        for post in &mut posts {
            let previous_mentions = post.ticker_mentions.clone();
            post.ticker_mentions =
                self.x_ticker_mentions_for_text(&post.text, now_ms, &previous_mentions);
        }
        self.x_feed.posts = posts;
    }

    pub(crate) fn fill_missing_x_ticker_reference_prices(&mut self, now_ms: u64) {
        if self.x_feed.posts.is_empty() {
            return;
        }

        let mut posts = std::mem::take(&mut self.x_feed.posts);
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
        self.x_feed.posts = posts;
    }

    fn x_ticker_mentions_for_text(
        &self,
        text: &str,
        reference_seen_ms: u64,
        previous_mentions: &[XTickerMention],
    ) -> Vec<XTickerMention> {
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
                    if mention.reference_price.is_none() {
                        mention.reference_price = self.resolve_mid_for_symbol(&matched.symbol_key);
                        if mention.reference_price.is_some() {
                            mention.reference_seen_ms = reference_seen_ms;
                        }
                    }
                    mention
                } else {
                    XTickerMention {
                        reference_price: self.resolve_mid_for_symbol(&matched.symbol_key),
                        reference_seen_ms,
                        symbol: matched.symbol_key,
                        ticker: matched.ticker,
                    }
                }
            })
            .collect()
    }

    fn push_new_x_post_alerts(&mut self, posts: &[XFeedPost]) {
        const MAX_ALERTS_PER_REFRESH: usize = 3;

        for post in posts.iter().take(MAX_ALERTS_PER_REFRESH) {
            self.push_x_feed_alert(x_post_alert_message(post));
        }

        if posts.len() > MAX_ALERTS_PER_REFRESH {
            self.push_x_feed_alert(format!(
                "{} more X posts",
                posts.len() - MAX_ALERTS_PER_REFRESH
            ));
        }
    }
}

fn x_post_alert_message(post: &XFeedPost) -> String {
    const MAX_PREVIEW_CHARS: usize = 140;
    let preview = ellipsized_text(
        post.text.lines().next().unwrap_or_default(),
        MAX_PREVIEW_CHARS,
    );

    if preview.is_empty() {
        format!("@{} posted on X", post.username)
    } else {
        format!("@{}: {}", post.username, preview)
    }
}
