use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::telegram_feed::{
    TELEGRAM_AVATAR_RETRY_BACKOFF_MS, TELEGRAM_FEED_MAX_CHANNELS, TelegramFeedPage,
    TelegramFeedPost, fetch_telegram_avatar_bytes, fetch_telegram_channel_posts,
    normalize_public_channel_input,
};
use iced::Task;
use iced::widget::image::Handle as ImageHandle;

impl TradingTerminal {
    pub(crate) fn update_telegram_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshTelegramFeed => self.request_telegram_feed_refresh(),
            Message::TelegramFeedRefreshTick => self.request_telegram_feed_background_refresh(),
            Message::TelegramFeedLoaded(channel, result) => {
                return self.handle_telegram_feed_loaded(channel, *result);
            }
            Message::TelegramAvatarLoaded(channel, avatar_url, request_id, result) => {
                self.handle_telegram_avatar_loaded(channel, avatar_url, request_id, *result);
                Task::none()
            }
            Message::TelegramFeedChannelInputChanged(input) => {
                self.telegram_feed.channel_input = input;
                Task::none()
            }
            Message::TelegramFeedAddChannel => self.add_telegram_feed_channel(),
            Message::TelegramFeedRemoveChannel(channel) => {
                self.remove_telegram_feed_channel(&channel);
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
        self.request_telegram_feed_refresh_with_visibility(false)
    }

    fn request_telegram_feed_refresh_with_visibility(&mut self, visible: bool) -> Task<Message> {
        let channels = self.telegram_feed.channels.clone();
        if channels.is_empty() {
            self.telegram_feed.last_error = Some("Add a public Telegram channel".to_string());
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

        if self.telegram_feed.channels.len() >= TELEGRAM_FEED_MAX_CHANNELS {
            self.telegram_feed.last_error = Some(format!(
                "Telegram Feed supports up to {TELEGRAM_FEED_MAX_CHANNELS} channels"
            ));
            return Task::none();
        }

        self.telegram_feed.channels.push(channel.clone());
        self.telegram_feed.channel_input.clear();
        self.telegram_feed.last_error = None;
        self.persist_config();
        self.telegram_feed.loading_channels.push(channel.clone());
        self.request_telegram_channel_refresh_task(channel)
    }

    fn remove_telegram_feed_channel(&mut self, channel: &str) {
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

        if !self
            .telegram_feed
            .channels
            .iter()
            .any(|existing| existing == &channel)
        {
            return Task::none();
        }

        match result {
            Ok(page) => {
                let now_ms = Self::now_ms();
                let avatar_task = self.store_telegram_channel_profile(page.profile);
                self.telegram_feed.last_error = None;
                let had_existing_posts = self
                    .telegram_feed
                    .posts
                    .iter()
                    .any(|post| post.channel == channel);
                let mut new_posts = Vec::new();
                for mut post in page.posts {
                    if let Some(existing_post) =
                        self.telegram_feed.posts.iter_mut().find(|existing| {
                            existing.channel == channel && existing.message_id == post.message_id
                        })
                    {
                        existing_post.text = post.text;
                        existing_post.timestamp_ms = post.timestamp_ms;
                        existing_post.url = post.url;
                    } else {
                        if had_existing_posts {
                            post.first_seen_ms = now_ms;
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

    fn store_telegram_channel_profile(
        &mut self,
        mut profile: crate::telegram_feed::TelegramChannelProfile,
    ) -> Task<Message> {
        let now_ms = Self::now_ms();
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
        if !self
            .telegram_feed
            .channels
            .iter()
            .any(|existing| existing == &channel)
        {
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
    let mut preview = post.text.lines().next().unwrap_or_default().to_string();
    const MAX_PREVIEW_CHARS: usize = 140;
    if preview.chars().count() > MAX_PREVIEW_CHARS {
        preview = preview.chars().take(MAX_PREVIEW_CHARS - 3).collect();
        preview.push_str("...");
    }

    if preview.is_empty() {
        format!("@{} posted a new message", post.channel)
    } else {
        format!("@{}: {}", post.channel, preview)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeroseneConfig;

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
    fn adding_channels_is_capped() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.telegram_feed.channels = (0..TELEGRAM_FEED_MAX_CHANNELS)
            .map(|index| format!("channel_{index}"))
            .collect();
        terminal.telegram_feed.channel_input = "another_channel".to_string();

        let _task = terminal.update_telegram_feed(Message::TelegramFeedAddChannel);

        assert_eq!(
            terminal.telegram_feed.channels.len(),
            TELEGRAM_FEED_MAX_CHANNELS
        );
        assert!(
            terminal
                .telegram_feed
                .last_error
                .as_deref()
                .is_some_and(|error| error.contains("supports up to"))
        );
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
}
