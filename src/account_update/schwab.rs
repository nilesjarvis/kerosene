use crate::account_state::ActiveAccountSource;
use crate::app_state::TradingTerminal;
use crate::message::{Message, SchwabAccountsMessageResult, SchwabTokenRefreshMessageResult};
use crate::schwab::{
    SchwabOAuthTokenRefresh, fetch_schwab_accounts_snapshot, refresh_schwab_access_token,
};
use iced::Task;
use zeroize::{Zeroize, Zeroizing};

impl TradingTerminal {
    pub(super) fn update_schwab(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SchwabClientIdChanged(input) => {
                self.schwab.client_id_input.zeroize();
                self.schwab.client_id_input = input.into_zeroizing().into();
                Task::none()
            }
            Message::SchwabClientSecretChanged(input) => {
                self.schwab.client_secret_input.zeroize();
                self.schwab.client_secret_input = input.into_zeroizing().into();
                Task::none()
            }
            Message::SchwabAccessTokenChanged(input) => {
                self.schwab.access_token_input.zeroize();
                self.schwab.access_token_input = input.into_zeroizing().into();
                Task::none()
            }
            Message::SchwabRefreshTokenChanged(input) => {
                self.schwab.refresh_token_input.zeroize();
                self.schwab.refresh_token_input = input.into_zeroizing().into();
                Task::none()
            }
            Message::SchwabConnect => self.connect_schwab(),
            Message::SchwabAccessTokenRefreshed(request_id, result) => {
                self.handle_schwab_access_token_refreshed(request_id, result.into_result())
            }
            Message::SchwabAccountsRefresh => self.request_schwab_accounts_refresh(),
            Message::SchwabAccountsLoaded(request_id, result) => {
                self.handle_schwab_accounts_loaded(request_id, result.into_result())
            }
            Message::SchwabAccountPickerSelected(hash) => match hash.into_option() {
                Some(hash) => self.select_schwab_account(hash),
                None => Task::none(),
            },
            Message::SchwabClearCredentials => {
                if !self.persist_schwab_credentials_secret_from_keys("", "", "", "") {
                    self.schwab.status = self.secret_store_status.clone();
                    return Task::none();
                }
                self.schwab.clear_credentials();
                if self.active_account_source == ActiveAccountSource::Schwab {
                    self.active_account_source = ActiveAccountSource::Hyperliquid;
                }
                self.persist_config();
                Task::none()
            }
            Message::SchwabTokenRefreshTick => self.maybe_auto_refresh_schwab_access_token(),
            _ => Task::none(),
        }
    }

    fn maybe_auto_refresh_schwab_access_token(&mut self) -> Task<Message> {
        let now_ms = Self::now_ms();
        if !self.schwab.has_refresh_credentials()
            || self.schwab.token_refreshing
            || !self.schwab.access_token_refresh_due(now_ms)
            || !self.schwab.auto_token_refresh_attempt_allowed(now_ms)
        {
            return Task::none();
        }
        self.schwab.record_auto_token_refresh_attempt(now_ms);
        self.request_schwab_access_token_refresh()
    }

    fn connect_schwab(&mut self) -> Task<Message> {
        if self.schwab.has_refresh_credential_input() {
            let Some((client_id, client_secret, refresh_token)) =
                self.schwab.refresh_credentials_candidate_from_input()
            else {
                return Task::none();
            };
            return self.start_schwab_access_token_refresh(client_id, client_secret, refresh_token);
        }

        if !self.schwab.access_token_input.trim().is_empty() {
            return self.connect_schwab_with_access_token_input();
        }

        if self.schwab.has_refresh_credentials() {
            return self.request_schwab_access_token_refresh();
        }

        if self.schwab.has_access_token() {
            return self.request_schwab_accounts_refresh();
        }

        self.schwab.status = Some((
            "Paste Schwab app credentials plus refresh token, or paste an access token".to_string(),
            true,
        ));
        Task::none()
    }

    fn connect_schwab_with_access_token_input(&mut self) -> Task<Message> {
        let Some(access_token) = self.schwab.access_token_candidate_from_input() else {
            return Task::none();
        };
        let (client_id, client_secret, _old_access_token, refresh_token) =
            self.schwab.oauth_credentials_for_secret();
        if !self.persist_schwab_credentials_secret_from_keys(
            client_id.as_str(),
            client_secret.as_str(),
            access_token.as_str(),
            refresh_token.as_str(),
        ) {
            self.schwab.status = self.secret_store_status.clone();
            return Task::none();
        }
        self.schwab.commit_oauth_credentials(
            client_id.as_str(),
            client_secret.as_str(),
            access_token.as_str(),
            refresh_token.as_str(),
            None,
        );
        self.persist_config();
        self.request_schwab_accounts_refresh()
    }

    fn request_schwab_access_token_refresh(&mut self) -> Task<Message> {
        if !self.schwab.has_refresh_credentials() {
            self.schwab.status = Some((
                "Save Schwab app key, app secret, and refresh token before refreshing".to_string(),
                true,
            ));
            return Task::none();
        }
        if self.schwab.token_refreshing {
            return Task::none();
        }
        self.start_schwab_access_token_refresh(
            self.schwab.client_id_for_task(),
            self.schwab.client_secret_for_task(),
            self.schwab.refresh_token_for_task(),
        )
    }

    fn start_schwab_access_token_refresh(
        &mut self,
        client_id: Zeroizing<String>,
        client_secret: Zeroizing<String>,
        refresh_token: Zeroizing<String>,
    ) -> Task<Message> {
        if self.schwab.token_refreshing {
            return Task::none();
        }
        let request_id = self.schwab.next_token_refresh_request_id();
        self.schwab.token_refreshing = true;
        self.schwab.status = Some(("Refreshing Schwab access token".to_string(), false));
        Task::perform(
            refresh_schwab_access_token(client_id, client_secret, refresh_token),
            move |result| {
                Message::SchwabAccessTokenRefreshed(
                    request_id,
                    SchwabTokenRefreshMessageResult::new(result),
                )
            },
        )
    }

    fn handle_schwab_access_token_refreshed(
        &mut self,
        request_id: u64,
        result: Result<SchwabOAuthTokenRefresh, String>,
    ) -> Task<Message> {
        if request_id != self.schwab.token_refresh_request_id {
            return Task::none();
        }
        self.schwab.token_refreshing = false;

        match result {
            Ok(refresh) => {
                let (client_id, client_secret, fallback_refresh_token) = self
                    .schwab
                    .pending_refresh_credentials_for_secret()
                    .unwrap_or_else(|| {
                        (
                            self.schwab.client_id_for_task(),
                            self.schwab.client_secret_for_task(),
                            self.schwab.refresh_token_for_task(),
                        )
                    });
                let refresh_token = refresh
                    .refresh_token
                    .unwrap_or_else(|| Zeroizing::new(fallback_refresh_token.as_str().to_string()));
                let expires_at_ms = refresh
                    .expires_in_secs
                    .map(|secs| Self::now_ms().saturating_add(secs.saturating_mul(1_000)));

                if !self.persist_schwab_credentials_secret_from_keys(
                    client_id.as_str(),
                    client_secret.as_str(),
                    refresh.access_token.as_str(),
                    refresh_token.as_str(),
                ) {
                    self.schwab.clear_pending_refresh_credentials();
                    self.schwab.status = self.secret_store_status.clone();
                    return Task::none();
                }

                self.schwab.commit_oauth_credentials(
                    client_id.as_str(),
                    client_secret.as_str(),
                    refresh.access_token.as_str(),
                    refresh_token.as_str(),
                    expires_at_ms,
                );
                self.persist_config();
                self.request_schwab_accounts_refresh()
            }
            Err(error) => {
                self.schwab.clear_pending_refresh_credentials();
                self.schwab.status = Some((error, true));
                Task::none()
            }
        }
    }

    fn request_schwab_accounts_refresh(&mut self) -> Task<Message> {
        let now_ms = Self::now_ms();
        if (self.schwab.access_token_refresh_due(now_ms) || !self.schwab.has_access_token())
            && self.schwab.has_refresh_credentials()
        {
            return self.request_schwab_access_token_refresh();
        }
        if !self.schwab.has_access_token() {
            self.schwab.status = Some((
                "Connect Schwab before refreshing accounts".to_string(),
                true,
            ));
            return Task::none();
        }
        if self.schwab.accounts_loading {
            return Task::none();
        }
        let request_id = self.schwab.next_accounts_request_id();
        self.schwab.accounts_loading = true;
        self.schwab.status = Some(("Loading Schwab accounts".to_string(), false));
        let token = self.schwab.access_token_for_task();
        Task::perform(fetch_schwab_accounts_snapshot(token), move |result| {
            Message::SchwabAccountsLoaded(request_id, SchwabAccountsMessageResult::new(result))
        })
    }

    fn handle_schwab_accounts_loaded(
        &mut self,
        request_id: u64,
        result: Result<crate::schwab::SchwabAccountsSnapshot, String>,
    ) -> Task<Message> {
        if request_id != self.schwab.accounts_request_id {
            return Task::none();
        }
        self.schwab.accounts_loading = false;
        match result {
            Ok(snapshot) => {
                self.schwab.apply_accounts_snapshot(snapshot);
                let count = self.schwab.connected_account_count();
                if count == 0 {
                    self.schwab.status = Some((
                        "Schwab connected, but no accounts were returned".to_string(),
                        true,
                    ));
                } else {
                    self.schwab.status = Some((
                        format!("Schwab connected; {count} account(s) loaded"),
                        false,
                    ));
                    if self.active_account_source == ActiveAccountSource::Schwab
                        && self.schwab.selected_account_hash.is_none()
                    {
                        self.active_account_source = ActiveAccountSource::Hyperliquid;
                    }
                }
                Task::none()
            }
            Err(error) => {
                self.schwab.status = Some((error, true));
                Task::none()
            }
        }
    }

    fn select_schwab_account(&mut self, hash: String) -> Task<Message> {
        let known = self
            .schwab
            .accounts
            .iter()
            .any(|account| account.account_hash == hash)
            || self
                .schwab
                .linked_accounts
                .iter()
                .any(|account| account.hash_value == hash);
        if !known {
            self.schwab.status = Some(("Schwab account is no longer available".to_string(), true));
            return Task::none();
        }

        self.schwab.selected_account_hash = Some(hash);
        self.active_account_source = ActiveAccountSource::Schwab;
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        Task::none()
    }
}
