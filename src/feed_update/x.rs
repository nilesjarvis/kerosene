use crate::app_state::TradingTerminal;
use crate::message::{
    Message, XAuthContextMessageResult, XFeedPageMessageResult, XListsMessageResult,
};
use crate::pane_state::PaneKind;
use crate::x_feed::{
    XAuthenticatedUser, XFeedId, XFeedInstance, XFeedPage, XFeedRequestError, XFeedSource,
    XListSummary, fetch_x_auth_context, fetch_x_feed_page, fetch_x_lists,
};
use iced::Task;
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
                self.x_feed.clear_access_token();
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
                self.handle_x_feed_loaded(source, request_id, result.into_result());
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn connect_x_feed(&mut self) -> Task<Message> {
        let Some(token) = self.x_feed.save_access_token_from_input() else {
            return Task::none();
        };
        let request_id = self.x_feed.next_connect_request_id();
        self.x_feed.connecting = true;
        self.x_feed.status = Some(("Connecting to X".to_string(), false));
        Task::perform(fetch_x_auth_context(token), move |result| {
            Message::XFeedAuthLoaded(request_id, XAuthContextMessageResult::new(result))
        })
    }

    fn handle_x_feed_auth_loaded(
        &mut self,
        request_id: u64,
        result: Result<(XAuthenticatedUser, Vec<XListSummary>), String>,
    ) -> Task<Message> {
        if request_id != self.x_feed.connect_request_id {
            return Task::none();
        }
        self.x_feed.connecting = false;
        match result {
            Ok((user, lists)) => {
                let username = user.username.clone();
                let list_count = lists.len();
                self.x_feed.auth_user = Some(user);
                self.x_feed.lists = lists;
                self.x_feed.status = Some((
                    format!("Connected @{username}; {list_count} Lists available"),
                    false,
                ));
                self.request_x_feed_open_refresh(true)
            }
            Err(err) => {
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
        result: Result<Vec<XListSummary>, String>,
    ) {
        if request_id != self.x_feed.lists_request_id {
            return;
        }
        self.x_feed.lists_loading = false;
        match result {
            Ok(lists) => {
                let count = lists.len();
                self.x_feed.lists = lists;
                self.x_feed.status = Some((format!("Loaded {count} X Lists"), false));
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
    ) {
        if !self.x_feed.finish_source_refresh(&source, request_id) {
            return;
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
            }
        }
    }
}
