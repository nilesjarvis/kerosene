use crate::app_state::TradingTerminal;
use crate::message::{
    Message, XAuthContextMessageResult, XFeedPageMessageResult, XListsMessageResult,
    XProfileImageMessageResult,
};
use crate::pane_state::PaneKind;
use crate::x_feed::{
    X_PROFILE_IMAGE_RETRY_BACKOFF_MS, XAuthenticatedUser, XAuthorProfile, XFeedId, XFeedInstance,
    XFeedPage, XFeedPost, XFeedRequestError, XFeedSource, XListsFetchOutcome, fetch_x_auth_context,
    fetch_x_feed_page, fetch_x_lists, fetch_x_profile_image_bytes,
};
use iced::Task;
use iced::widget::image::Handle as ImageHandle;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(crate) fn update_x_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::XFeedAccessTokenChanged(input) => {
                self.x_feed.access_token_input.zeroize();
                self.x_feed.access_token_input = input.into_zeroizing().into();
                Task::none()
            }
            Message::XFeedConnect => self.connect_x_feed(),
            Message::XFeedAuthLoaded(request_id, result) => {
                self.handle_x_feed_auth_loaded(request_id, result.into_result())
            }
            Message::XFeedClearAccessToken => {
                if !self.persist_x_access_token_secret_from_key("") {
                    self.x_feed.status = self.secret_store_status.clone();
                    return Task::none();
                }
                self.x_feed.clear_access_token();
                self.persist_config();
                Task::none()
            }
            Message::XFeedListsRefresh => self.request_x_feed_lists_refresh(),
            Message::XFeedListsLoaded(request_id, result) => {
                self.handle_x_feed_lists_loaded(request_id, result.into_result());
                Task::none()
            }
            Message::XFeedSourceSelected(id, option) => {
                if let Some(instance) = self.x_feed.instances.get_mut(&id) {
                    instance.source = option.source;
                    instance.posts.clear();
                    instance.last_error = None;
                    instance.last_refresh_ms = None;
                    self.persist_config();
                    return self.request_x_feed_refresh(id, true);
                }
                Task::none()
            }
            Message::RefreshXFeed(id) => self.request_x_feed_refresh(id, true),
            Message::XFeedRefreshTick => self.request_x_feed_open_refresh(false),
            Message::XFeedLoaded(source, request_id, result) => {
                self.handle_x_feed_loaded(source, request_id, result.into_result())
            }
            Message::XProfileImageLoaded(request_id, result) => {
                self.handle_x_profile_image_loaded(request_id, result.into_result());
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn connect_x_feed(&mut self) -> Task<Message> {
        let Some(token) = self.x_feed.access_token_candidate_from_input() else {
            return Task::none();
        };
        let request_id = self.x_feed.next_connect_request_id();
        self.x_feed.connecting = true;
        self.x_feed.status = Some(("Connecting to X".to_string(), false));
        Task::perform(fetch_x_auth_context(token), move |result| {
            Message::XFeedAuthLoaded(request_id, XAuthContextMessageResult::new(result))
        })
    }

    pub(crate) fn request_x_feed_auth_refresh(&mut self) -> Task<Message> {
        if !self.x_feed.has_access_token() {
            return Task::none();
        }
        if self.x_feed.connecting {
            return Task::none();
        }
        let request_id = self.x_feed.next_connect_request_id();
        self.x_feed.connecting = true;
        self.x_feed.status = Some(("Connecting to X".to_string(), false));
        let token = self.x_feed.access_token_for_task();
        Task::perform(fetch_x_auth_context(token), move |result| {
            Message::XFeedAuthLoaded(request_id, XAuthContextMessageResult::new(result))
        })
    }

    fn handle_x_feed_auth_loaded(
        &mut self,
        request_id: u64,
        result: Result<(XAuthenticatedUser, XListsFetchOutcome), String>,
    ) -> Task<Message> {
        if request_id != self.x_feed.connect_request_id {
            return Task::none();
        }
        self.x_feed.connecting = false;
        match result {
            Ok((user, outcome)) => {
                if let Some(token) = self.x_feed.pending_access_token_for_secret() {
                    if !self.persist_x_access_token_secret_from_key(token.as_str()) {
                        self.x_feed.clear_pending_access_token();
                        self.x_feed.status = self.secret_store_status.clone();
                        return Task::none();
                    }
                    self.x_feed.commit_access_token(token.as_str());
                    self.persist_config();
                }
                let username = user.username.clone();
                let list_count = outcome.lists.len();
                let status_suffix = outcome.status_suffix();
                self.x_feed.auth_user = Some(user);
                self.x_feed.lists = outcome.lists;
                self.x_feed.status = Some((
                    format!("Connected @{username}; {list_count} Lists available{status_suffix}"),
                    false,
                ));
                self.request_x_feed_open_refresh(true)
            }
            Err(err) => {
                self.x_feed.clear_pending_access_token();
                self.x_feed.status = Some((err, true));
                Task::none()
            }
        }
    }

    fn request_x_feed_lists_refresh(&mut self) -> Task<Message> {
        let Some(user_id) = self.x_feed.auth_user.as_ref().map(|user| user.id.clone()) else {
            self.x_feed.status = Some(("Connect X before refreshing Lists".to_string(), true));
            return Task::none();
        };
        if !self.x_feed.has_access_token() {
            self.x_feed.status = Some(("Paste an X access token first".to_string(), true));
            return Task::none();
        }

        let token = self.x_feed.access_token_for_task();
        let request_id = self.x_feed.next_lists_request_id();
        self.x_feed.lists_loading = true;
        self.x_feed.status = Some(("Refreshing X Lists".to_string(), false));
        Task::perform(fetch_x_lists(token, user_id), move |result| {
            Message::XFeedListsLoaded(request_id, XListsMessageResult::new(result))
        })
    }

    fn handle_x_feed_lists_loaded(
        &mut self,
        request_id: u64,
        result: Result<XListsFetchOutcome, String>,
    ) {
        if request_id != self.x_feed.lists_request_id {
            return;
        }
        self.x_feed.lists_loading = false;
        match result {
            Ok(outcome) => {
                let count = outcome.lists.len();
                let status_suffix = outcome.status_suffix();
                self.x_feed.lists = outcome.lists;
                self.x_feed
                    .status
                    .replace((format!("Loaded {count} X Lists{status_suffix}"), false));
            }
            Err(err) => self.x_feed.status = Some((err, true)),
        }
    }

    pub(crate) fn request_x_feed_open_refresh(&mut self, visible: bool) -> Task<Message> {
        let open_ids = self
            .panes
            .iter()
            .filter_map(|(_, kind)| match kind {
                PaneKind::XFeed(id) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>();
        let tasks = open_ids
            .into_iter()
            .map(|id| self.request_x_feed_refresh(id, visible))
            .collect::<Vec<_>>();
        Task::batch(tasks)
    }

    pub(crate) fn request_x_feed_refresh(&mut self, id: XFeedId, visible: bool) -> Task<Message> {
        if !self.x_feed.has_access_token() {
            if visible && let Some(instance) = self.x_feed.instances.get_mut(&id) {
                instance.last_error = Some("Paste an X OAuth 2.0 user access token".to_string());
            }
            return Task::none();
        }
        let Some(user_id) = self.x_feed.auth_user.as_ref().map(|user| user.id.clone()) else {
            if self.x_feed.has_access_token() {
                if visible && let Some(instance) = self.x_feed.instances.get_mut(&id) {
                    instance.last_error = None;
                }
                return self.request_x_feed_auth_refresh();
            }
            if visible && let Some(instance) = self.x_feed.instances.get_mut(&id) {
                instance.last_error = Some("Connect X before refreshing".to_string());
            }
            return Task::none();
        };
        let Some(instance) = self.x_feed.instances.get(&id) else {
            return Task::none();
        };
        let source = instance.source.clone();
        let now_ms = Self::now_ms();
        if let Some(reset_ms) = self.x_feed.source_rate_limited_until(&source, now_ms) {
            let status = format!(
                "X rate limit reached for {}; reset {}",
                source.label(),
                crate::helpers::format_timestamp(reset_ms / 1_000)
            );
            if visible && let Some(instance) = self.x_feed.instances.get_mut(&id) {
                instance.last_error = Some(status.clone());
            }
            self.x_feed.status = Some((status, true));
            return Task::none();
        }
        if self.x_feed.source_refresh_in_flight(&source) {
            return Task::none();
        }

        let since_id = source
            .supports_since_id()
            .then(|| {
                self.x_feed
                    .instances
                    .values()
                    .filter(|instance| instance.source == source)
                    .filter_map(XFeedInstance::newest_seen_id)
                    .filter_map(|id| id.parse::<u64>().ok().map(|parsed| (parsed, id)))
                    .max_by_key(|(parsed, _)| *parsed)
                    .map(|(_, id)| id)
            })
            .flatten();
        let token = self.x_feed.access_token_for_task();
        let request_id = self.x_feed.begin_source_refresh(&source);
        Task::perform(
            fetch_x_feed_page(token, user_id, source.clone(), since_id),
            move |result| {
                Message::XFeedLoaded(
                    source.clone(),
                    request_id,
                    XFeedPageMessageResult::new(result),
                )
            },
        )
    }

    fn handle_x_feed_loaded(
        &mut self,
        source: XFeedSource,
        request_id: u64,
        result: Result<XFeedPage, XFeedRequestError>,
    ) -> Task<Message> {
        if !self.x_feed.finish_source_refresh(&source, request_id) {
            return Task::none();
        }

        let now_ms = Self::now_ms();
        match result {
            Ok(page) => {
                if let Some(reset_ms) = page.rate_limited_until_ms {
                    self.x_feed.set_source_rate_limit(&source, reset_ms);
                }
                let mut applied_any = false;
                for instance in self
                    .x_feed
                    .instances
                    .values_mut()
                    .filter(|instance| instance.source == source)
                {
                    instance.apply_page(&page, now_ms);
                    applied_any = true;
                }
                if applied_any {
                    self.x_feed.status = Some((
                        format!("X Feed updated · {} posts", page.posts.len()),
                        false,
                    ));
                }
                self.schedule_x_profile_image_fetches(&page)
            }
            Err(err) => {
                if let Some(reset_ms) = err.rate_limited_until_ms {
                    self.x_feed.set_source_rate_limit(&source, reset_ms);
                }
                for instance in self
                    .x_feed
                    .instances
                    .values_mut()
                    .filter(|instance| instance.source == source)
                {
                    instance.last_error = Some(err.message.clone());
                }
                self.x_feed.status = Some((err.message, true));
                Task::none()
            }
        }
    }

    fn schedule_x_profile_image_fetches(&mut self, page: &XFeedPage) -> Task<Message> {
        let now_ms = Self::now_ms();
        let mut tasks = Vec::new();

        for post in &page.posts {
            let Some(image_url) = post.author_profile_image_url.clone() else {
                self.store_x_author_profile_metadata(post);
                continue;
            };
            let key = post.author_profile_key();
            let mut profile = self
                .x_feed
                .author_profiles
                .remove(&key)
                .unwrap_or_else(|| XAuthorProfile::from_post(post));

            profile.author_id = post.author_id.clone();
            profile.username = post.author_username.clone();
            profile.name = post.author_name.clone();
            profile.initials = post.author_initials();
            if profile.profile_image_url.as_deref() != Some(image_url.as_str()) {
                profile.profile_image_url = Some(image_url.clone());
                profile.image_handle = None;
                profile.image_loading_url = None;
                profile.image_request_id = 0;
                profile.image_failed_at_ms = None;
            }

            let should_fetch = profile.image_handle.is_none()
                && profile.image_loading_url.as_deref() != Some(image_url.as_str())
                && !profile.image_failed_at_ms.is_some_and(|failed_at_ms| {
                    now_ms.saturating_sub(failed_at_ms) < X_PROFILE_IMAGE_RETRY_BACKOFF_MS
                });
            if should_fetch {
                self.x_feed.next_profile_image_request_id =
                    self.x_feed.next_profile_image_request_id.saturating_add(1);
                profile.image_request_id = self.x_feed.next_profile_image_request_id;
                profile.image_loading_url = Some(image_url.clone());
                profile.image_failed_at_ms = None;
                let request_id = profile.image_request_id;
                tasks.push(Task::perform(
                    fetch_x_profile_image_bytes(image_url),
                    move |result| {
                        Message::XProfileImageLoaded(
                            request_id,
                            XProfileImageMessageResult::new(result),
                        )
                    },
                ));
            }

            self.x_feed.author_profiles.insert(key, profile);
        }

        Task::batch(tasks)
    }

    fn store_x_author_profile_metadata(&mut self, post: &XFeedPost) {
        let key = post.author_profile_key();
        let profile = self
            .x_feed
            .author_profiles
            .entry(key)
            .or_insert_with(|| XAuthorProfile::from_post(post));
        profile.author_id = post.author_id.clone();
        profile.username = post.author_username.clone();
        profile.name = post.author_name.clone();
        profile.initials = post.author_initials();
    }

    fn handle_x_profile_image_loaded(&mut self, request_id: u64, result: Result<Vec<u8>, String>) {
        let Some(profile) = self
            .x_feed
            .author_profiles
            .values_mut()
            .find(|profile| profile.image_request_id == request_id)
        else {
            return;
        };

        if profile.image_loading_url.is_none() {
            return;
        }
        profile.image_loading_url = None;

        match result {
            Ok(bytes) => {
                profile.image_handle = Some(ImageHandle::from_bytes(bytes));
                profile.image_request_id = 0;
                profile.image_failed_at_ms = None;
            }
            Err(_) => {
                profile.image_handle = None;
                profile.image_request_id = 0;
                profile.image_failed_at_ms = Some(Self::now_ms());
            }
        }
    }
}
