use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::ellipsized_text;
use crate::message::Message;
use crate::x_feed::{
    X_FEED_MAX_SOURCES, XFeedPage, XFeedPost, XFeedStreamEvent, XTickerMention,
    fetch_x_recent_posts, normalize_x_bearer_token_input, normalize_x_handle_input,
};
use crate::x_feed_stream::clear_x_stream_rules;
use iced::Task;
use zeroize::Zeroize;
use zeroize::Zeroizing;

impl TradingTerminal {
    pub(crate) fn update_x_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshXFeed => self.request_x_feed_refresh(),
            Message::XFeedRefreshTick => self.request_x_feed_background_refresh(),
            Message::XFeedLoaded(request_id, handles, result) => {
                self.handle_x_feed_recent_loaded(request_id, &handles, *result)
            }
            Message::XFeedStreamEvent(nonce, event) => {
                self.handle_x_feed_stream_event(nonce, event)
            }
            Message::XFeedRuleCleanupFinished {
                cleanup_through_generation,
                reconnect_nonce,
                result,
            } => self.handle_x_feed_rule_cleanup_finished(
                cleanup_through_generation,
                reconnect_nonce,
                *result,
            ),
            Message::XFeedBearerTokenChanged(input) => {
                self.x_feed.bearer_token_input.zeroize();
                self.x_feed.bearer_token_input = input.into_zeroizing().into();
                Task::none()
            }
            Message::SaveXFeedBearerToken => self.save_x_feed_bearer_token(),
            Message::XFeedSourceInputChanged(input) => {
                self.x_feed.source_input = input;
                Task::none()
            }
            Message::XFeedAddSource => self.add_x_feed_source(),
            Message::XFeedRemoveSource(handle) => self.remove_x_feed_source(&handle),
            Message::ToggleXFeedStreaming => {
                let cleanup_generation = self.x_feed.stream_reconnect_nonce;
                let cleanup_token = self.x_feed.bearer_token.clone().into_zeroizing();
                let was_streaming_enabled = self.x_feed.streaming_enabled;
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
                if was_streaming_enabled && !self.x_feed.streaming_enabled {
                    Self::x_feed_stream_rule_cleanup_task_for_token(
                        cleanup_token,
                        cleanup_generation,
                        self.x_feed.stream_reconnect_nonce,
                    )
                } else {
                    Task::none()
                }
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
        if self.x_feed.refreshing() {
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

        let request_id = self.x_feed.next_refresh_request_id();
        let handles = self.x_feed.handles.clone();
        self.request_x_feed_refresh_task(request_id, handles)
    }

    fn request_x_feed_refresh_task(&self, request_id: u64, handles: Vec<String>) -> Task<Message> {
        let token = zeroize::Zeroizing::new(self.x_feed.bearer_token.trim().to_string());
        let request_handles = handles.clone();
        Task::perform(fetch_x_recent_posts(token, handles), move |result| {
            Message::XFeedLoaded(request_id, request_handles.clone(), Box::new(result))
        })
    }

    fn save_x_feed_bearer_token(&mut self) -> Task<Message> {
        let previous_token =
            Zeroizing::new(normalize_x_bearer_token_input(&self.x_feed.bearer_token));
        let cleanup_generation = self.x_feed.stream_reconnect_nonce;
        let saved_token = Zeroizing::new(normalize_x_bearer_token_input(
            &self.x_feed.bearer_token_input,
        ));
        if !self.persist_x_secret_from_token(saved_token.as_str()) {
            return Task::none();
        }

        self.x_feed.bearer_token.zeroize();
        self.x_feed.bearer_token = saved_token.as_str().to_string().into();
        self.x_feed.bearer_token_input.zeroize();
        self.x_feed.bearer_token_input = self.x_feed.bearer_token.clone();
        self.x_feed.invalidate_refresh_requests();
        self.x_feed.stream_connected = false;
        self.x_feed.stream_reconnect_nonce = self.x_feed.stream_reconnect_nonce.saturating_add(1);
        self.x_feed.stream_status = if self.x_feed.bearer_token.trim().is_empty() {
            Some(("X bearer token cleared".to_string(), false))
        } else {
            Some(("X bearer token saved".to_string(), false))
        };
        self.persist_config();

        let cleanup_task =
            if !previous_token.is_empty() && previous_token.as_str() != saved_token.as_str() {
                Self::x_feed_stream_rule_cleanup_task_for_token(
                    previous_token,
                    cleanup_generation,
                    self.x_feed.stream_reconnect_nonce,
                )
            } else {
                Task::none()
            };
        let refresh_task = if !saved_token.is_empty() && self.x_feed.posts.is_empty() {
            self.request_x_feed_refresh()
        } else {
            Task::none()
        };
        Task::batch([cleanup_task, refresh_task])
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

    fn remove_x_feed_source(&mut self, handle: &str) -> Task<Message> {
        let Ok(handle) = normalize_x_handle_input(handle) else {
            return Task::none();
        };
        let cleanup_generation = self.x_feed.stream_reconnect_nonce;
        let cleanup_token = self.x_feed.bearer_token.clone().into_zeroizing();
        self.x_feed.handles.retain(|existing| existing != &handle);
        self.x_feed.posts.retain(|post| post.username != handle);
        self.x_feed.prune_profiles_to_visible_posts();
        self.x_feed.clear_seen_posts();
        self.x_feed.invalidate_refresh_requests();
        self.x_feed.last_error = None;
        self.x_feed.stream_connected = false;
        self.x_feed.stream_reconnect_nonce = self.x_feed.stream_reconnect_nonce.saturating_add(1);
        self.persist_config();
        if self.x_feed.handles.is_empty() {
            Self::x_feed_stream_rule_cleanup_task_for_token(
                cleanup_token,
                cleanup_generation,
                self.x_feed.stream_reconnect_nonce,
            )
        } else {
            Task::none()
        }
    }

    fn handle_x_feed_stream_event(
        &mut self,
        reconnect_nonce: u64,
        event: XFeedStreamEvent,
    ) -> Task<Message> {
        if reconnect_nonce != self.x_feed.stream_reconnect_nonce || !self.x_feed.streaming_enabled {
            return Task::none();
        }

        match event {
            XFeedStreamEvent::Status { connected, message } => {
                self.x_feed.stream_connected = connected;
                self.x_feed.stream_status = Some((message, !connected));
                Task::none()
            }
            XFeedStreamEvent::Loaded(result) => self.handle_x_feed_loaded(*result, false),
        }
    }

    fn handle_x_feed_rule_cleanup_finished(
        &mut self,
        cleanup_through_generation: u64,
        reconnect_nonce: u64,
        result: Result<(), String>,
    ) -> Task<Message> {
        if reconnect_nonce != self.x_feed.stream_reconnect_nonce {
            return Task::none();
        }

        if let Err(err) = result {
            if cleanup_through_generation < self.x_feed.stream_reconnect_nonce
                && self.x_feed.streaming_enabled
                && !self.x_feed.bearer_token.trim().is_empty()
                && !self.x_feed.handles.is_empty()
            {
                return Task::none();
            }
            self.x_feed.stream_connected = false;
            self.x_feed.stream_status =
                Some((format!("X stream rule cleanup failed: {err}"), true));
        }
        Task::none()
    }

    pub(crate) fn x_feed_stream_rule_cleanup_task_for_token(
        bearer_token: Zeroizing<String>,
        cleanup_through_generation: u64,
        result_reconnect_nonce: u64,
    ) -> Task<Message> {
        let bearer_token = Zeroizing::new(normalize_x_bearer_token_input(&bearer_token));
        if bearer_token.is_empty() {
            return Task::none();
        }

        Task::perform(
            clear_x_stream_rules(bearer_token, cleanup_through_generation),
            move |result| Message::XFeedRuleCleanupFinished {
                cleanup_through_generation,
                reconnect_nonce: result_reconnect_nonce,
                result: Box::new(result),
            },
        )
    }

    fn handle_x_feed_recent_loaded(
        &mut self,
        request_id: u64,
        handles: &[String],
        result: Result<XFeedPage, String>,
    ) -> Task<Message> {
        if request_id != self.x_feed.refresh_request_id || handles != self.x_feed.handles.as_slice()
        {
            if request_id == self.x_feed.refresh_request_id {
                self.x_feed.loading = false;
                self.x_feed.background_loading = false;
            }
            return Task::none();
        }

        self.handle_x_feed_loaded(result, true)
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
                let page = self.x_feed_page_for_current_handles(page);
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
                self.x_feed.prune_profiles_to_visible_posts();
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

    fn x_feed_page_for_current_handles(&self, mut page: XFeedPage) -> XFeedPage {
        page.posts.retain(|post| {
            normalize_x_handle_input(&post.username)
                .is_ok_and(|handle| self.x_feed.handles.iter().any(|current| current == &handle))
        });
        let author_ids: std::collections::HashSet<&str> = page
            .posts
            .iter()
            .map(|post| post.author_id.as_str())
            .collect();
        page.profiles
            .retain(|author_id, _| author_ids.contains(author_id.as_str()));
        page
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::{TradingTerminal, sensitive_string};
    use crate::config;
    use crate::x_feed::{X_FEED_RENDER_LIMIT, XFeedAuthorProfile, parse_x_stream_page};

    use std::collections::HashMap;

    #[test]
    fn recent_refresh_result_applies_when_request_context_matches() {
        let mut terminal = terminal_with_x_source("marketfeed");
        let request_id = terminal.x_feed.next_refresh_request_id();
        terminal.x_feed.loading = true;

        let _task = terminal.update_x_feed(Message::XFeedLoaded(
            request_id,
            vec!["marketfeed".to_string()],
            Box::new(Ok(sample_x_page("marketfeed"))),
        ));

        assert!(!terminal.x_feed.loading);
        assert_eq!(terminal.x_feed.posts.len(), 1);
        assert_eq!(terminal.x_feed.posts[0].username, "marketfeed");
        assert!(terminal.x_feed.last_refresh_ms.is_some());
    }

    #[test]
    fn stale_recent_refresh_result_is_ignored_after_source_change() {
        let mut terminal = terminal_with_x_source("marketfeed");
        let stale_request_id = terminal.x_feed.next_refresh_request_id();
        terminal.x_feed.loading = true;
        terminal.x_feed.handles.clear();
        terminal.x_feed.invalidate_refresh_requests();

        let _task = terminal.update_x_feed(Message::XFeedLoaded(
            stale_request_id,
            vec!["marketfeed".to_string()],
            Box::new(Ok(sample_x_page("marketfeed"))),
        ));

        assert!(terminal.x_feed.posts.is_empty());
        assert!(terminal.x_feed.profiles.is_empty());
        assert!(terminal.x_feed.last_refresh_ms.is_none());
        assert!(!terminal.x_feed.loading);
    }

    #[test]
    fn background_refresh_tick_is_ignored_while_refresh_is_in_flight() {
        let mut terminal = terminal_with_x_source("marketfeed");

        let _task = terminal.update_x_feed(Message::XFeedRefreshTick);
        let first_request_id = terminal.x_feed.refresh_request_id;
        let _task = terminal.update_x_feed(Message::XFeedRefreshTick);

        assert_eq!(first_request_id, 1);
        assert_eq!(terminal.x_feed.refresh_request_id, first_request_id);
        assert!(terminal.x_feed.background_loading);
    }

    #[test]
    fn manual_refresh_can_supersede_background_refresh() {
        let mut terminal = terminal_with_x_source("marketfeed");

        let _task = terminal.update_x_feed(Message::XFeedRefreshTick);
        let background_request_id = terminal.x_feed.refresh_request_id;
        let _task = terminal.update_x_feed(Message::RefreshXFeed);

        assert_eq!(background_request_id, 1);
        assert_eq!(
            terminal.x_feed.refresh_request_id,
            background_request_id + 1
        );
        assert!(terminal.x_feed.loading);
    }

    #[test]
    fn stale_stream_status_is_ignored_after_stream_is_disabled() {
        let mut terminal = terminal_with_x_source("marketfeed");
        terminal.x_feed.streaming_enabled = true;
        let stale_nonce = terminal.x_feed.stream_reconnect_nonce;

        let _task = terminal.update_x_feed(Message::ToggleXFeedStreaming);
        let _task = terminal.update_x_feed(Message::XFeedStreamEvent(
            stale_nonce,
            XFeedStreamEvent::Status {
                connected: true,
                message: "old stream connected".to_string(),
            },
        ));

        assert!(!terminal.x_feed.streaming_enabled);
        assert!(!terminal.x_feed.stream_connected);
        assert_eq!(
            terminal
                .x_feed
                .stream_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("X stream disabled", false))
        );
    }

    #[test]
    fn stale_stream_loaded_is_ignored_after_source_removed() {
        let mut terminal = terminal_with_x_source("marketfeed");
        terminal.x_feed.streaming_enabled = true;
        let stale_nonce = terminal.x_feed.stream_reconnect_nonce;

        let _task = terminal.update_x_feed(Message::XFeedRemoveSource("marketfeed".to_string()));
        let _task = terminal.update_x_feed(Message::XFeedStreamEvent(
            stale_nonce,
            XFeedStreamEvent::Loaded(Box::new(Ok(sample_x_page("marketfeed")))),
        ));

        assert!(terminal.x_feed.handles.is_empty());
        assert!(terminal.x_feed.posts.is_empty());
        assert!(terminal.x_feed.profiles.is_empty());
        assert!(terminal.x_feed.last_refresh_ms.is_none());
    }

    #[test]
    fn stream_loaded_filters_posts_for_unselected_handles_without_alerting() {
        let mut terminal = terminal_with_x_source("marketfeed");
        terminal.desktop_notifications = false;
        terminal.sound_enabled = false;
        terminal.x_feed.streaming_enabled = true;
        terminal.x_feed.notifications_enabled = true;
        terminal.x_feed.record_seen_post("already-seen");
        let nonce = terminal.x_feed.stream_reconnect_nonce;

        let _task = terminal.update_x_feed(Message::XFeedStreamEvent(
            nonce,
            XFeedStreamEvent::Loaded(Box::new(Ok(sample_x_page("removedsource")))),
        ));

        assert!(terminal.x_feed.posts.is_empty());
        assert!(terminal.x_feed.profiles.is_empty());
        assert!(terminal.toasts.is_empty());
        assert!(terminal.x_feed.last_refresh_ms.is_some());
    }

    #[test]
    fn stream_error_display_uses_sanitized_parser_message() {
        let mut terminal = terminal_with_x_source("marketfeed");
        terminal.x_feed.streaming_enabled = true;
        let nonce = terminal.x_feed.stream_reconnect_nonce;
        let json = br#"{
            "errors": [{
                "detail": "Authorization: Bearer x-secret-token api_key=\"abc123\" trace=0123456789abcdef0123456789abcdef01234567"
            }]
        }"#;
        let error = parse_x_stream_page(json, 1).expect_err("stream error");

        let _task = terminal.update_x_feed(Message::XFeedStreamEvent(
            nonce,
            XFeedStreamEvent::Loaded(Box::new(Err(error))),
        ));

        let last_error = terminal.x_feed.last_error.as_deref().expect("last error");
        assert!(last_error.contains("<redacted>"));
        assert!(last_error.contains("<redacted-hex>"));
        assert!(!last_error.contains("x-secret-token"));
        assert!(!last_error.contains("abc123"));
        assert!(!last_error.contains("0123456789abcdef0123456789abcdef01234567"));
    }

    #[test]
    fn current_rule_cleanup_failure_updates_stream_status() {
        let mut terminal = terminal_with_x_source("marketfeed");
        let nonce = terminal.x_feed.stream_reconnect_nonce;

        let _task = terminal.update_x_feed(Message::XFeedRuleCleanupFinished {
            cleanup_through_generation: nonce,
            reconnect_nonce: nonce,
            result: Box::new(Err("upstream unavailable".to_string())),
        });

        assert_eq!(
            terminal
                .x_feed
                .stream_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("X stream rule cleanup failed: upstream unavailable", true))
        );
    }

    #[test]
    fn stale_rule_cleanup_failure_is_ignored_after_stream_restart() {
        let mut terminal = terminal_with_x_source("marketfeed");
        let stale_nonce = terminal.x_feed.stream_reconnect_nonce;
        terminal.x_feed.stream_reconnect_nonce += 1;
        terminal.x_feed.stream_status = Some(("X stream connected".to_string(), false));

        let _task = terminal.update_x_feed(Message::XFeedRuleCleanupFinished {
            cleanup_through_generation: stale_nonce,
            reconnect_nonce: stale_nonce,
            result: Box::new(Err("old cleanup failed".to_string())),
        });

        assert_eq!(
            terminal
                .x_feed
                .stream_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("X stream connected", false))
        );
    }

    #[test]
    fn obsolete_rule_cleanup_failure_does_not_disconnect_replacement_stream() {
        let mut terminal = terminal_with_x_source("marketfeed");
        terminal.x_feed.streaming_enabled = true;
        terminal.x_feed.stream_connected = true;
        terminal.x_feed.stream_reconnect_nonce = 8;
        terminal.x_feed.stream_status = Some(("X stream connected".to_string(), false));

        let _task = terminal.update_x_feed(Message::XFeedRuleCleanupFinished {
            cleanup_through_generation: 7,
            reconnect_nonce: 8,
            result: Box::new(Err("old cleanup failed".to_string())),
        });

        assert!(terminal.x_feed.stream_connected);
        assert_eq!(
            terminal
                .x_feed
                .stream_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("X stream connected", false))
        );
    }

    #[test]
    fn removing_x_source_prunes_cached_profiles() {
        let mut terminal = terminal_with_x_source("marketfeed");
        let page = sample_x_page("marketfeed");
        terminal.x_feed.profiles = page.profiles;
        terminal.x_feed.posts = page.posts;

        let _task = terminal.update_x_feed(Message::XFeedRemoveSource("marketfeed".to_string()));

        assert!(terminal.x_feed.posts.is_empty());
        assert!(terminal.x_feed.profiles.is_empty());
    }

    #[test]
    fn loaded_x_page_prunes_profiles_for_truncated_posts() {
        let mut terminal = terminal_with_x_source("marketfeed");
        let request_id = terminal.x_feed.next_refresh_request_id();
        terminal.x_feed.loading = true;

        let _task = terminal.update_x_feed(Message::XFeedLoaded(
            request_id,
            vec!["marketfeed".to_string()],
            Box::new(Ok(sample_x_page_with_count(
                "marketfeed",
                X_FEED_RENDER_LIMIT + 3,
            ))),
        ));

        assert_eq!(terminal.x_feed.posts.len(), X_FEED_RENDER_LIMIT);
        assert_eq!(terminal.x_feed.profiles.len(), X_FEED_RENDER_LIMIT);
        assert!(!terminal.x_feed.profiles.contains_key("author-0"));
        assert!(terminal.x_feed.profiles.contains_key("author-3"));
    }

    #[test]
    fn save_x_token_locked_encrypted_credentials_keeps_live_stream_state() {
        let mut terminal = terminal_with_x_source("marketfeed");
        configure_encrypted_x_token(&mut terminal, "old-token", false);
        terminal.x_feed.bearer_token = sensitive_string("old-token");
        terminal.x_feed.bearer_token_input = sensitive_string("new-token");
        terminal.x_feed.stream_connected = true;
        terminal.x_feed.stream_reconnect_nonce = 7;
        terminal.x_feed.stream_status = Some(("X stream connected".to_string(), false));
        terminal.x_feed.loading = true;
        terminal.x_feed.background_loading = true;
        terminal.config_save_due_at = None;

        let _task = terminal.update_x_feed(Message::SaveXFeedBearerToken);

        assert_eq!(terminal.x_feed.bearer_token.as_str(), "old-token");
        assert_eq!(terminal.x_feed.bearer_token_input.as_str(), "new-token");
        assert!(terminal.x_feed.stream_connected);
        assert_eq!(terminal.x_feed.stream_reconnect_nonce, 7);
        assert_eq!(
            terminal
                .x_feed
                .stream_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("X stream connected", false))
        );
        assert!(terminal.x_feed.loading);
        assert!(terminal.x_feed.background_loading);
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Unlock encrypted credentials"));
    }

    #[test]
    fn clear_x_token_locked_encrypted_credentials_keeps_live_token_and_payload() {
        let mut terminal = terminal_with_x_source("marketfeed");
        configure_encrypted_x_token(&mut terminal, "old-token", false);
        terminal.x_feed.bearer_token = sensitive_string("old-token");
        terminal.x_feed.bearer_token_input = sensitive_string("");
        terminal.x_feed.stream_connected = true;
        terminal.x_feed.stream_reconnect_nonce = 7;
        terminal.x_feed.stream_status = Some(("X stream connected".to_string(), false));
        terminal.config_save_due_at = None;

        let _task = terminal.update_x_feed(Message::SaveXFeedBearerToken);

        assert_eq!(terminal.x_feed.bearer_token.as_str(), "old-token");
        assert_eq!(terminal.x_feed.bearer_token_input.as_str(), "");
        assert!(terminal.x_feed.stream_connected);
        assert_eq!(terminal.x_feed.stream_reconnect_nonce, 7);
        assert_eq!(
            terminal
                .x_feed
                .stream_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("X stream connected", false))
        );
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should remain present"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(payload.global_x_bearer_token(), "old-token");
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
    }

    #[test]
    fn save_x_token_commits_after_encrypted_persistence_succeeds() {
        let mut terminal = terminal_with_x_source("marketfeed");
        configure_encrypted_x_token(&mut terminal, "old-token", true);
        terminal.x_feed.bearer_token = sensitive_string("old-token");
        terminal.x_feed.bearer_token_input = sensitive_string("  new-token  ");
        terminal.x_feed.stream_connected = true;
        terminal.x_feed.stream_reconnect_nonce = 7;
        terminal.config_save_due_at = None;

        let _task = terminal.update_x_feed(Message::SaveXFeedBearerToken);

        assert_eq!(terminal.x_feed.bearer_token.as_str(), "new-token");
        assert_eq!(terminal.x_feed.bearer_token_input.as_str(), "new-token");
        assert!(!terminal.x_feed.stream_connected);
        assert_eq!(terminal.x_feed.stream_reconnect_nonce, 8);
        assert_eq!(
            terminal
                .x_feed
                .stream_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("X bearer token saved", false))
        );
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should be rewritten"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(payload.global_x_bearer_token(), "new-token");
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn clear_x_token_commits_after_encrypted_persistence_succeeds() {
        let mut terminal = terminal_with_x_source("marketfeed");
        configure_encrypted_x_token(&mut terminal, "old-token", true);
        terminal.x_feed.bearer_token = sensitive_string("old-token");
        terminal.x_feed.bearer_token_input = sensitive_string("   ");
        terminal.x_feed.stream_connected = true;
        terminal.x_feed.stream_reconnect_nonce = 7;
        terminal.config_save_due_at = None;

        let _task = terminal.update_x_feed(Message::SaveXFeedBearerToken);

        assert_eq!(terminal.x_feed.bearer_token.as_str(), "");
        assert_eq!(terminal.x_feed.bearer_token_input.as_str(), "");
        assert!(!terminal.x_feed.stream_connected);
        assert_eq!(terminal.x_feed.stream_reconnect_nonce, 8);
        assert_eq!(
            terminal
                .x_feed
                .stream_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("X bearer token cleared", false))
        );
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should be rewritten"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(payload.global_x_bearer_token(), "");
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_some());
    }

    fn terminal_with_x_source(handle: &str) -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.x_feed.bearer_token = sensitive_string("test-token");
        terminal.x_feed.handles = vec![handle.to_string()];
        terminal
    }

    fn configure_encrypted_x_token(terminal: &mut TradingTerminal, token: &str, unlocked: bool) {
        terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
        terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
        terminal.encrypted_secret_password = sensitive_string("test-password");
        terminal.encrypted_secrets = Some(
            config::encrypt_secrets(
                &config::SecretPayload::from_credentials(&[], "", "", token),
                &terminal.encrypted_secret_password,
            )
            .expect("test encrypted payload"),
        );
        terminal.encrypted_secrets_unlocked = unlocked;
        terminal.secret_migration_save_blocked = false;
        terminal.secret_store_status = None;
    }

    fn sample_x_page(username: &str) -> XFeedPage {
        let author_id = "author-1".to_string();
        let mut profiles = HashMap::new();
        profiles.insert(
            author_id.clone(),
            XFeedAuthorProfile {
                id: author_id.clone(),
                username: username.to_string(),
                name: "Market Feed".to_string(),
                initials: "MF".to_string(),
                verified: false,
                avatar_url: None,
            },
        );

        XFeedPage {
            profiles,
            posts: vec![XFeedPost {
                id: format!("{username}-post-1"),
                author_id,
                username: username.to_string(),
                text: "BTC update".to_string(),
                timestamp_ms: 1_000,
                fetched_at_ms: 1_100,
                request_started_ms: 1_000,
                request_duration_ms: 100,
                first_seen_ms: 0,
                url: format!("https://x.com/{username}/status/1"),
                ticker_mentions: Vec::new(),
            }],
        }
    }

    fn sample_x_page_with_count(username: &str, count: usize) -> XFeedPage {
        let mut profiles = HashMap::new();
        let mut posts = Vec::new();
        for index in 0..count {
            let author_id = format!("author-{index}");
            profiles.insert(
                author_id.clone(),
                XFeedAuthorProfile {
                    id: author_id.clone(),
                    username: username.to_string(),
                    name: format!("Market Feed {index}"),
                    initials: "MF".to_string(),
                    verified: false,
                    avatar_url: None,
                },
            );
            posts.push(XFeedPost {
                id: format!("{username}-post-{index}"),
                author_id,
                username: username.to_string(),
                text: "BTC update".to_string(),
                timestamp_ms: index as u64,
                fetched_at_ms: 1_100,
                request_started_ms: 1_000,
                request_duration_ms: 100,
                first_seen_ms: 0,
                url: format!("https://x.com/{username}/status/{index}"),
                ticker_mentions: Vec::new(),
            });
        }
        XFeedPage { profiles, posts }
    }
}
