use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::{
    Message, XAccessTokenRefreshMessageResult, XAuthContextMessageResult, XFeedPageMessageResult,
    XListsMessageResult, XProfileImageMessageResult,
};
use crate::pane_state::PaneKind;
use crate::x_feed::{
    X_PROFILE_IMAGE_RETRY_BACKOFF_MS, XAuthorProfile, XFeedId, XFeedInstance, XFeedPage, XFeedPost,
    XFeedSource, fetch_x_auth_context, fetch_x_feed_page, fetch_x_lists,
    fetch_x_profile_image_bytes, refresh_x_access_token,
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
            Message::XFeedOAuthClientIdChanged(input) => {
                self.x_feed.oauth_client_id_input.zeroize();
                self.x_feed.oauth_client_id_input = input.into_zeroizing().into();
                Task::none()
            }
            Message::XFeedRefreshTokenChanged(input) => {
                self.x_feed.refresh_token_input.zeroize();
                self.x_feed.refresh_token_input = input.into_zeroizing().into();
                Task::none()
            }
            Message::XFeedConnect => self.connect_x_feed(),
            Message::XAccessTokenRefreshed(request_id, result) => {
                self.handle_x_access_token_refreshed(request_id, result)
            }
            Message::XFeedAuthLoaded(request_id, result) => {
                self.handle_x_feed_auth_loaded(request_id, result)
            }
            Message::XFeedClearAccessToken => {
                if !self.persist_x_oauth_credentials_secret_from_keys("", "", "") {
                    self.x_feed.status = self.secret_store_status.clone();
                    return Task::none();
                }
                self.x_feed.clear_access_token();
                self.persist_config();
                Task::none()
            }
            Message::XFeedListsRefresh => self.request_x_feed_lists_refresh(),
            Message::XFeedListsLoaded(request_id, result) => {
                self.handle_x_feed_lists_loaded(request_id, result);
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
                self.handle_x_feed_loaded(source, request_id, result)
            }
            Message::XProfileImageLoaded(request_id, result) => {
                self.handle_x_profile_image_loaded(request_id, result);
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn connect_x_feed(&mut self) -> Task<Message> {
        if self.x_feed.has_refresh_credential_input() {
            let Some((client_id, refresh_token)) =
                self.x_feed.refresh_credentials_candidate_from_input()
            else {
                return Task::none();
            };
            return self.start_x_access_token_refresh(client_id, refresh_token);
        }

        let Some(token) = self.x_feed.access_token_candidate_from_input() else {
            return Task::none();
        };
        self.start_x_feed_auth_request(token)
    }

    fn start_x_feed_auth_request(&mut self, token: zeroize::Zeroizing<String>) -> Task<Message> {
        let Some(request_id) = self.x_feed.begin_auth_request() else {
            return Task::none();
        };
        self.x_feed.status = Some(("Connecting to X".to_string(), false));
        Task::perform(fetch_x_auth_context(token), move |result| {
            Message::XFeedAuthLoaded(request_id, XAuthContextMessageResult::new(result))
        })
    }

    pub(crate) fn request_x_feed_auth_refresh(&mut self) -> Task<Message> {
        if self.x_feed.connecting || self.x_feed.token_refreshing {
            return Task::none();
        }
        let now_ms = Self::now_ms();
        if self.x_feed.access_token_refresh_due(now_ms)
            || (!self.x_feed.has_access_token() && self.x_feed.has_refresh_credentials())
        {
            return self.request_x_access_token_refresh();
        }
        if !self.x_feed.has_access_token() {
            return Task::none();
        }
        let token = self.x_feed.access_token_for_task();
        self.start_x_feed_auth_request(token)
    }

    fn request_x_access_token_refresh(&mut self) -> Task<Message> {
        if !self.x_feed.has_refresh_credentials() {
            return Task::none();
        }
        if self.x_feed.connecting || self.x_feed.token_refreshing {
            return Task::none();
        }
        let client_id = self.x_feed.oauth_client_id_for_task();
        let refresh_token = self.x_feed.refresh_token_for_task();
        self.start_x_access_token_refresh(client_id, refresh_token)
    }

    fn start_x_access_token_refresh(
        &mut self,
        client_id: zeroize::Zeroizing<String>,
        refresh_token: zeroize::Zeroizing<String>,
    ) -> Task<Message> {
        let Some(request_id) = self
            .x_feed
            .begin_token_refresh_request(client_id.as_str(), refresh_token.as_str())
        else {
            return Task::none();
        };
        self.x_feed.status = Some(("Refreshing X token".to_string(), false));
        Task::perform(
            refresh_x_access_token(client_id, refresh_token),
            move |result| {
                Message::XAccessTokenRefreshed(
                    request_id,
                    XAccessTokenRefreshMessageResult::new(result),
                )
            },
        )
    }

    fn handle_x_access_token_refreshed(
        &mut self,
        request_id: u64,
        result: XAccessTokenRefreshMessageResult,
    ) -> Task<Message> {
        let Some((client_id, fallback_refresh_token)) =
            self.x_feed.finish_token_refresh_request(request_id)
        else {
            return Task::none();
        };
        self.x_feed.clear_pending_access_token();

        match result.into_result() {
            Ok(refresh) => {
                let refresh_token = refresh.refresh_token.unwrap_or(fallback_refresh_token);
                let expires_at_ms = refresh
                    .expires_in_secs
                    .map(|secs| Self::now_ms().saturating_add(secs.saturating_mul(1_000)));

                if !self.persist_x_oauth_credentials_secret_from_keys(
                    refresh.access_token.as_str(),
                    client_id.as_str(),
                    refresh_token.as_str(),
                ) {
                    self.x_feed.clear_pending_oauth_credentials();
                    self.x_feed.status = self.secret_store_status.clone();
                    return Task::none();
                }

                self.x_feed.commit_oauth_credentials(
                    refresh.access_token.as_str(),
                    client_id.as_str(),
                    refresh_token.as_str(),
                    expires_at_ms,
                );
                self.persist_config();
                let token = self.x_feed.access_token_for_task();
                self.start_x_feed_auth_request(token)
            }
            Err(err) => {
                self.x_feed.clear_pending_oauth_credentials();
                self.x_feed.status = Some((redact_sensitive_response_text(&err), true));
                Task::none()
            }
        }
    }

    fn handle_x_feed_auth_loaded(
        &mut self,
        request_id: u64,
        result: XAuthContextMessageResult,
    ) -> Task<Message> {
        let Some(owns_lists_result) = self.x_feed.finish_auth_request(request_id) else {
            return Task::none();
        };
        match result.into_result() {
            Ok((user, outcome)) => {
                let mut credentials_changed = false;
                if let Some(token) = self.x_feed.pending_access_token_for_secret() {
                    if !self.persist_x_oauth_credentials_secret_from_keys(token.as_str(), "", "") {
                        self.x_feed.clear_pending_access_token();
                        self.x_feed.status = self.secret_store_status.clone();
                        return Task::none();
                    }
                    credentials_changed = self.x_feed.commit_access_token(token.as_str());
                    self.persist_config();
                }
                self.x_feed.clear_pending_oauth_credentials();
                let username = user.username.clone();
                let list_count = outcome.lists.len();
                let status_suffix = outcome.status_suffix();
                self.x_feed.auth_user = Some(user);
                if owns_lists_result || credentials_changed {
                    self.x_feed.lists = outcome.lists;
                }
                self.x_feed.status = Some((
                    format!("Connected @{username}; {list_count} Lists available{status_suffix}"),
                    false,
                ));
                self.request_x_feed_open_refresh(true)
            }
            Err(err) => {
                self.x_feed.clear_pending_access_token();
                self.x_feed.status = Some((redact_sensitive_response_text(&err), true));
                Task::none()
            }
        }
    }

    fn request_x_feed_lists_refresh(&mut self) -> Task<Message> {
        let now_ms = Self::now_ms();
        if self.x_feed.access_token_refresh_due(now_ms)
            || (!self.x_feed.has_access_token() && self.x_feed.has_refresh_credentials())
        {
            return self.request_x_access_token_refresh();
        }
        let Some(user_id) = self.x_feed.auth_user.as_ref().map(|user| user.id.clone()) else {
            self.x_feed.status = Some(("Connect X before refreshing Lists".to_string(), true));
            return Task::none();
        };
        if !self.x_feed.has_access_token() {
            self.x_feed.status = Some(("Paste an X access token first".to_string(), true));
            return Task::none();
        }

        let token = self.x_feed.access_token_for_task();
        let request_id = self.x_feed.begin_lists_request(&user_id);
        self.x_feed.status = Some(("Refreshing X Lists".to_string(), false));
        Task::perform(fetch_x_lists(token, user_id), move |result| {
            Message::XFeedListsLoaded(request_id, XListsMessageResult::new(result))
        })
    }

    fn handle_x_feed_lists_loaded(&mut self, request_id: u64, result: XListsMessageResult) {
        let current_user_id = self.x_feed.auth_user.as_ref().map(|user| user.id.clone());
        if !self
            .x_feed
            .finish_lists_request(request_id, current_user_id.as_deref())
        {
            return;
        }
        match result.into_result() {
            Ok(outcome) => {
                let count = outcome.lists.len();
                let status_suffix = outcome.status_suffix();
                self.x_feed.lists = outcome.lists;
                self.x_feed
                    .status
                    .replace((format!("Loaded {count} X Lists{status_suffix}"), false));
            }
            Err(err) => {
                self.x_feed.status = Some((redact_sensitive_response_text(&err), true));
            }
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
        let now_ms = Self::now_ms();
        if self.x_feed.access_token_refresh_due(now_ms)
            || (!self.x_feed.has_access_token() && self.x_feed.has_refresh_credentials())
        {
            return self.request_x_access_token_refresh();
        }
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
        let request_id = self.x_feed.begin_source_refresh(&source, &user_id);
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
        result: XFeedPageMessageResult,
    ) -> Task<Message> {
        let current_user_id = self.x_feed.auth_user.as_ref().map(|user| user.id.clone());
        if !self
            .x_feed
            .finish_source_refresh(&source, request_id, current_user_id.as_deref())
        {
            return Task::none();
        }

        let now_ms = Self::now_ms();
        match result.into_result() {
            Ok(page) => {
                if page.source != source {
                    return Task::none();
                }
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
            Err(mut err) => {
                err.message = redact_sensitive_response_text(&err.message);
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
                self.x_feed
                    .cancel_profile_image_request(profile.image_request_id);
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
                profile.image_request_id =
                    self.x_feed.begin_profile_image_request(&key, &image_url);
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

    fn handle_x_profile_image_loaded(
        &mut self,
        request_id: u64,
        result: XProfileImageMessageResult,
    ) {
        let Some((profile_key, image_url)) = self.x_feed.finish_profile_image_request(request_id)
        else {
            return;
        };
        let Some(profile) = self.x_feed.author_profiles.get_mut(&profile_key) else {
            return;
        };

        if profile.image_request_id != request_id
            || profile.image_loading_url.as_deref() != Some(image_url.as_str())
        {
            return;
        }
        profile.image_loading_url = None;

        match result.into_result() {
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

#[cfg(test)]
mod tests;
