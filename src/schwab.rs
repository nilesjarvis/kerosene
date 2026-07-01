use crate::api::{CLIENT, Candle, KEROSENE_USER_AGENT};
use crate::app_state::{SensitiveString, sensitive_string};
use crate::helpers::redact_sensitive_response_text;
use crate::timeframe::Timeframe;
use base64::Engine as _;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use zeroize::{Zeroize, Zeroizing};

const SCHWAB_TOKEN_URL: &str = "https://api.schwabapi.com/v1/oauth/token";
const SCHWAB_TRADER_BASE: &str = "https://api.schwabapi.com/trader/v1";
const SCHWAB_MARKET_DATA_BASE: &str = "https://api.schwabapi.com/marketdata/v1";
const SCHWAB_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const SCHWAB_SYMBOL_PREFIX: &str = "schwab:";
const SCHWAB_AUTO_TOKEN_REFRESH_RETRY_MS: u64 = 5 * 60 * 1_000;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SchwabOAuthTokenRefresh {
    pub(crate) access_token: Zeroizing<String>,
    pub(crate) refresh_token: Option<Zeroizing<String>>,
    pub(crate) expires_in_secs: Option<u64>,
}

impl fmt::Debug for SchwabOAuthTokenRefresh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchwabOAuthTokenRefresh")
            .field("access_token", &"<redacted>")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<redacted>"),
            )
            .field("expires_in_secs", &self.expires_in_secs)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SchwabLinkedAccount {
    pub(crate) account_number: Option<String>,
    pub(crate) hash_value: String,
}

impl fmt::Debug for SchwabLinkedAccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchwabLinkedAccount")
            .field(
                "account_number",
                &self.account_number.as_ref().map(|_| "<redacted>"),
            )
            .field("hash_value", &"<redacted>")
            .finish()
    }
}

impl SchwabLinkedAccount {
    pub(crate) fn masked_account_number(&self) -> String {
        self.account_number
            .as_deref()
            .map(mask_identifier)
            .unwrap_or_else(|| mask_identifier(&self.hash_value))
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct SchwabPositionSummary {
    pub(crate) symbol: String,
    pub(crate) quantity: f64,
    pub(crate) market_value: Option<f64>,
}

impl fmt::Debug for SchwabPositionSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchwabPositionSummary")
            .field("symbol", &self.symbol)
            .field("quantity", &"<redacted>")
            .field("market_value", &self.market_value.map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct SchwabAccountSummary {
    pub(crate) account_number: Option<String>,
    pub(crate) account_hash: String,
    pub(crate) account_type: Option<String>,
    pub(crate) cash_balance: Option<f64>,
    pub(crate) buying_power: Option<f64>,
    pub(crate) liquidation_value: Option<f64>,
    pub(crate) positions: Vec<SchwabPositionSummary>,
}

impl fmt::Debug for SchwabAccountSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchwabAccountSummary")
            .field(
                "account_number",
                &self.account_number.as_ref().map(|_| "<redacted>"),
            )
            .field("account_hash", &"<redacted>")
            .field("account_type", &self.account_type)
            .field("cash_balance", &self.cash_balance.map(|_| "<redacted>"))
            .field("buying_power", &self.buying_power.map(|_| "<redacted>"))
            .field(
                "liquidation_value",
                &self.liquidation_value.map(|_| "<redacted>"),
            )
            .field("positions", &self.positions.len())
            .finish()
    }
}

impl SchwabAccountSummary {
    pub(crate) fn masked_account_number(&self) -> String {
        self.account_number
            .as_deref()
            .map(mask_identifier)
            .unwrap_or_else(|| mask_identifier(&self.account_hash))
    }

    pub(crate) fn label(&self) -> String {
        let account = self.masked_account_number();
        match self
            .account_type
            .as_deref()
            .filter(|kind| !kind.trim().is_empty())
        {
            Some(kind) => format!("Schwab {kind} {account}"),
            None => format!("Schwab {account}"),
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct SchwabAccountsSnapshot {
    pub(crate) linked_accounts: Vec<SchwabLinkedAccount>,
    pub(crate) accounts: Vec<SchwabAccountSummary>,
}

impl fmt::Debug for SchwabAccountsSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchwabAccountsSnapshot")
            .field("linked_accounts", &self.linked_accounts)
            .field("accounts", &self.accounts)
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct SchwabState {
    pub(crate) client_id_input: SensitiveString,
    pub(crate) client_secret_input: SensitiveString,
    pub(crate) access_token_input: SensitiveString,
    pub(crate) refresh_token_input: SensitiveString,
    pending_client_id: SensitiveString,
    pending_client_secret: SensitiveString,
    pending_refresh_token: SensitiveString,
    client_id: SensitiveString,
    client_secret: SensitiveString,
    access_token: SensitiveString,
    refresh_token: SensitiveString,
    access_token_expires_at_ms: Option<u64>,
    last_auto_token_refresh_attempt_ms: Option<u64>,
    pub(crate) linked_accounts: Vec<SchwabLinkedAccount>,
    pub(crate) accounts: Vec<SchwabAccountSummary>,
    pub(crate) selected_account_hash: Option<String>,
    pub(crate) token_refresh_request_id: u64,
    pub(crate) accounts_request_id: u64,
    pub(crate) token_refreshing: bool,
    pub(crate) accounts_loading: bool,
    pub(crate) status: Option<(String, bool)>,
}

impl fmt::Debug for SchwabState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchwabState")
            .field("client_id_input", &"<redacted>")
            .field("client_secret_input", &"<redacted>")
            .field("access_token_input", &"<redacted>")
            .field("refresh_token_input", &"<redacted>")
            .field("pending_client_id", &"<redacted>")
            .field("pending_client_secret", &"<redacted>")
            .field("pending_refresh_token", &"<redacted>")
            .field("client_id", &"<redacted>")
            .field("client_secret", &"<redacted>")
            .field("access_token", &"<redacted>")
            .field("refresh_token", &"<redacted>")
            .field(
                "access_token_expires_at_ms",
                &self.access_token_expires_at_ms,
            )
            .field(
                "last_auto_token_refresh_attempt_ms",
                &self.last_auto_token_refresh_attempt_ms,
            )
            .field("linked_accounts", &self.linked_accounts.len())
            .field("accounts", &self.accounts.len())
            .field(
                "selected_account_hash",
                &self.selected_account_hash.as_ref().map(|_| "<redacted>"),
            )
            .field("token_refresh_request_id", &self.token_refresh_request_id)
            .field("accounts_request_id", &self.accounts_request_id)
            .field("token_refreshing", &self.token_refreshing)
            .field("accounts_loading", &self.accounts_loading)
            .field(
                "status",
                &self
                    .status
                    .as_ref()
                    .map(|(message, is_error)| (redact_sensitive_response_text(message), is_error)),
            )
            .finish()
    }
}

impl SchwabState {
    pub(crate) fn new(
        client_id: &str,
        client_secret: &str,
        access_token: &str,
        refresh_token: &str,
    ) -> Self {
        Self {
            client_id_input: sensitive_string(String::new()),
            client_secret_input: sensitive_string(String::new()),
            access_token_input: sensitive_string(String::new()),
            refresh_token_input: sensitive_string(String::new()),
            pending_client_id: sensitive_string(String::new()),
            pending_client_secret: sensitive_string(String::new()),
            pending_refresh_token: sensitive_string(String::new()),
            client_id: sensitive_string(client_id.trim().to_string()),
            client_secret: sensitive_string(client_secret.trim().to_string()),
            access_token: sensitive_string(access_token.trim().to_string()),
            refresh_token: sensitive_string(refresh_token.trim().to_string()),
            access_token_expires_at_ms: None,
            last_auto_token_refresh_attempt_ms: None,
            linked_accounts: Vec::new(),
            accounts: Vec::new(),
            selected_account_hash: None,
            token_refresh_request_id: 0,
            accounts_request_id: 0,
            token_refreshing: false,
            accounts_loading: false,
            status: None,
        }
    }

    pub(crate) fn has_access_token(&self) -> bool {
        !self.access_token.trim().is_empty()
    }

    pub(crate) fn has_refresh_credentials(&self) -> bool {
        !self.client_id.trim().is_empty()
            && !self.client_secret.trim().is_empty()
            && !self.refresh_token.trim().is_empty()
    }

    pub(crate) fn has_refresh_credential_input(&self) -> bool {
        !self.client_id_input.trim().is_empty()
            || !self.client_secret_input.trim().is_empty()
            || !self.refresh_token_input.trim().is_empty()
    }

    pub(crate) fn loading(&self) -> bool {
        self.token_refreshing || self.accounts_loading
    }

    pub(crate) fn access_token_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.access_token.trim().to_string())
    }

    pub(crate) fn client_id_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.client_id.trim().to_string())
    }

    pub(crate) fn client_secret_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.client_secret.trim().to_string())
    }

    pub(crate) fn refresh_token_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.refresh_token.trim().to_string())
    }

    pub(crate) fn oauth_credentials_for_secret(
        &self,
    ) -> (
        Zeroizing<String>,
        Zeroizing<String>,
        Zeroizing<String>,
        Zeroizing<String>,
    ) {
        (
            Zeroizing::new(self.client_id.trim().to_string()),
            Zeroizing::new(self.client_secret.trim().to_string()),
            Zeroizing::new(self.access_token.trim().to_string()),
            Zeroizing::new(self.refresh_token.trim().to_string()),
        )
    }

    pub(crate) fn refresh_credentials_candidate_from_input(
        &mut self,
    ) -> Option<(Zeroizing<String>, Zeroizing<String>, Zeroizing<String>)> {
        let client_id = self.client_id_input.trim().to_string();
        let client_secret = self.client_secret_input.trim().to_string();
        let refresh_token = self.refresh_token_input.trim().to_string();
        if client_id.is_empty() || client_secret.is_empty() || refresh_token.is_empty() {
            self.status = Some((
                "Paste a Schwab app key, app secret, and refresh token".to_string(),
                true,
            ));
            return None;
        }

        self.pending_client_id.zeroize();
        self.pending_client_secret.zeroize();
        self.pending_refresh_token.zeroize();
        self.pending_client_id = sensitive_string(client_id.clone());
        self.pending_client_secret = sensitive_string(client_secret.clone());
        self.pending_refresh_token = sensitive_string(refresh_token.clone());
        self.client_id_input.zeroize();
        self.client_secret_input.zeroize();
        self.refresh_token_input.zeroize();
        Some((
            Zeroizing::new(client_id),
            Zeroizing::new(client_secret),
            Zeroizing::new(refresh_token),
        ))
    }

    pub(crate) fn access_token_candidate_from_input(&mut self) -> Option<Zeroizing<String>> {
        let token = self.access_token_input.trim().to_string();
        if token.is_empty() {
            self.status = Some(("Paste a Schwab access token".to_string(), true));
            return None;
        }

        self.access_token_input.zeroize();
        Some(Zeroizing::new(token))
    }

    pub(crate) fn pending_refresh_credentials_for_secret(
        &self,
    ) -> Option<(Zeroizing<String>, Zeroizing<String>, Zeroizing<String>)> {
        let client_id = self.pending_client_id.trim().to_string();
        let client_secret = self.pending_client_secret.trim().to_string();
        let refresh_token = self.pending_refresh_token.trim().to_string();
        (!client_id.is_empty() && !client_secret.is_empty() && !refresh_token.is_empty()).then(
            || {
                (
                    Zeroizing::new(client_id),
                    Zeroizing::new(client_secret),
                    Zeroizing::new(refresh_token),
                )
            },
        )
    }

    pub(crate) fn clear_pending_refresh_credentials(&mut self) {
        self.pending_client_id.zeroize();
        self.pending_client_secret.zeroize();
        self.pending_refresh_token.zeroize();
    }

    pub(crate) fn commit_oauth_credentials(
        &mut self,
        client_id: &str,
        client_secret: &str,
        access_token: &str,
        refresh_token: &str,
        expires_at_ms: Option<u64>,
    ) -> bool {
        let changed = self.set_oauth_credentials_from_secret(
            client_id,
            client_secret,
            access_token,
            refresh_token,
            expires_at_ms,
        );
        self.client_id_input.zeroize();
        self.client_secret_input.zeroize();
        self.access_token_input.zeroize();
        self.refresh_token_input.zeroize();
        self.clear_pending_refresh_credentials();
        changed
    }

    pub(crate) fn set_oauth_credentials_from_secret(
        &mut self,
        client_id: &str,
        client_secret: &str,
        access_token: &str,
        refresh_token: &str,
        expires_at_ms: Option<u64>,
    ) -> bool {
        let client_id = client_id.trim();
        let client_secret = client_secret.trim();
        let access_token = access_token.trim();
        let refresh_token = refresh_token.trim();
        let changed = self.client_id.trim() != client_id
            || self.client_secret.trim() != client_secret
            || self.access_token.trim() != access_token
            || self.refresh_token.trim() != refresh_token;

        if changed {
            self.invalidate_requests();
            self.linked_accounts.clear();
            self.accounts.clear();
            self.selected_account_hash = None;
            self.last_auto_token_refresh_attempt_ms = None;
        }

        self.client_id.zeroize();
        self.client_secret.zeroize();
        self.access_token.zeroize();
        self.refresh_token.zeroize();
        self.client_id = sensitive_string(client_id.to_string());
        self.client_secret = sensitive_string(client_secret.to_string());
        self.access_token = sensitive_string(access_token.to_string());
        self.refresh_token = sensitive_string(refresh_token.to_string());
        self.access_token_expires_at_ms = expires_at_ms;
        changed
    }

    pub(crate) fn clear_credentials(&mut self) {
        self.client_id_input.zeroize();
        self.client_secret_input.zeroize();
        self.access_token_input.zeroize();
        self.refresh_token_input.zeroize();
        self.clear_pending_refresh_credentials();
        self.set_oauth_credentials_from_secret("", "", "", "", None);
        self.status = Some(("Schwab credentials cleared".to_string(), false));
    }

    pub(crate) fn access_token_refresh_due(&self, now_ms: u64) -> bool {
        if !self.has_refresh_credentials() {
            return false;
        }
        match self.access_token_expires_at_ms {
            Some(expires_at_ms) => expires_at_ms.saturating_sub(now_ms) <= 60_000,
            None => true,
        }
    }

    pub(crate) fn auto_token_refresh_attempt_allowed(&self, now_ms: u64) -> bool {
        self.last_auto_token_refresh_attempt_ms
            .is_none_or(|last| now_ms.saturating_sub(last) >= SCHWAB_AUTO_TOKEN_REFRESH_RETRY_MS)
    }

    pub(crate) fn record_auto_token_refresh_attempt(&mut self, now_ms: u64) {
        self.last_auto_token_refresh_attempt_ms = Some(now_ms);
    }

    pub(crate) fn next_token_refresh_request_id(&mut self) -> u64 {
        self.token_refresh_request_id = self.token_refresh_request_id.saturating_add(1);
        self.token_refresh_request_id
    }

    pub(crate) fn next_accounts_request_id(&mut self) -> u64 {
        self.accounts_request_id = self.accounts_request_id.saturating_add(1);
        self.accounts_request_id
    }

    pub(crate) fn invalidate_requests(&mut self) {
        self.token_refresh_request_id = self.token_refresh_request_id.saturating_add(1);
        self.accounts_request_id = self.accounts_request_id.saturating_add(1);
        self.token_refreshing = false;
        self.accounts_loading = false;
    }

    pub(crate) fn apply_accounts_snapshot(&mut self, snapshot: SchwabAccountsSnapshot) {
        self.linked_accounts = snapshot.linked_accounts;
        self.accounts = snapshot.accounts;
        if self.selected_account_hash.as_ref().is_none_or(|hash| {
            !self
                .accounts
                .iter()
                .any(|account| account.account_hash == *hash)
        }) {
            self.selected_account_hash = self
                .accounts
                .first()
                .map(|account| account.account_hash.clone())
                .or_else(|| {
                    self.linked_accounts
                        .first()
                        .map(|account| account.hash_value.clone())
                });
        }
    }

    pub(crate) fn selected_account_summary(&self) -> Option<&SchwabAccountSummary> {
        let hash = self.selected_account_hash.as_deref()?;
        self.accounts
            .iter()
            .find(|account| account.account_hash == hash)
    }

    pub(crate) fn selected_account_label(&self) -> String {
        self.selected_account_summary()
            .map(SchwabAccountSummary::label)
            .or_else(|| {
                let hash = self.selected_account_hash.as_deref()?;
                Some(format!("Schwab {}", mask_identifier(hash)))
            })
            .unwrap_or_else(|| "Schwab".to_string())
    }

    pub(crate) fn selected_account_line(&self) -> String {
        self.selected_account_summary()
            .map(|account| account.masked_account_number())
            .or_else(|| self.selected_account_hash.as_deref().map(mask_identifier))
            .unwrap_or_else(|| "No account selected".to_string())
    }

    pub(crate) fn connected_account_count(&self) -> usize {
        self.accounts.len().max(self.linked_accounts.len())
    }
}

pub(crate) fn schwab_symbol_key(symbol: &str) -> Option<String> {
    let symbol = normalize_schwab_symbol(symbol);
    (!symbol.is_empty()).then(|| format!("{SCHWAB_SYMBOL_PREFIX}{symbol}"))
}

pub(crate) fn schwab_symbol_from_key(key: &str) -> Option<&str> {
    key.strip_prefix(SCHWAB_SYMBOL_PREFIX)
        .filter(|symbol| !symbol.trim().is_empty())
}

pub(crate) fn is_schwab_symbol_key(key: &str) -> bool {
    schwab_symbol_from_key(key).is_some()
}

pub(crate) fn schwab_display_symbol(key: &str) -> Option<String> {
    schwab_symbol_from_key(key).map(str::to_string)
}

pub(crate) fn normalize_schwab_symbol(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '/'))
        .collect::<String>()
        .to_ascii_uppercase()
}

pub(crate) async fn refresh_schwab_access_token(
    client_id: Zeroizing<String>,
    client_secret: Zeroizing<String>,
    refresh_token: Zeroizing<String>,
) -> Result<SchwabOAuthTokenRefresh, String> {
    let credentials = Zeroizing::new(format!("{}:{}", client_id.trim(), client_secret.trim()));
    let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
    let response = CLIENT
        .post(SCHWAB_TOKEN_URL)
        .timeout(SCHWAB_REQUEST_TIMEOUT)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .header(AUTHORIZATION, format!("Basic {encoded}"))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.trim()),
        ])
        .send()
        .await
        .map_err(|e| format!("Schwab token refresh request failed: {e}"))?;

    parse_schwab_json_response::<SchwabTokenResponse>(response, "Schwab token refresh")
        .await
        .map(|response| SchwabOAuthTokenRefresh {
            access_token: response.access_token.into(),
            refresh_token: response.refresh_token.map(Into::into),
            expires_in_secs: response.expires_in,
        })
}

pub(crate) async fn fetch_schwab_accounts_snapshot(
    access_token: Zeroizing<String>,
) -> Result<SchwabAccountsSnapshot, String> {
    let linked_accounts = fetch_schwab_linked_accounts(access_token.clone()).await?;
    let account_number_to_hash: HashMap<String, String> = linked_accounts
        .iter()
        .filter_map(|account| {
            account
                .account_number
                .as_ref()
                .map(|number| (number.clone(), account.hash_value.clone()))
        })
        .collect();
    let accounts = fetch_schwab_accounts(access_token, &account_number_to_hash).await?;
    Ok(SchwabAccountsSnapshot {
        linked_accounts,
        accounts,
    })
}

pub(crate) async fn fetch_schwab_price_history(
    access_token: Zeroizing<String>,
    symbol_key: String,
    timeframe: Timeframe,
    start_time: u64,
    end_time: u64,
) -> Result<Vec<Candle>, String> {
    let Some(symbol) = schwab_symbol_from_key(&symbol_key).map(str::to_string) else {
        return Err("Schwab chart symbol is invalid".to_string());
    };
    let params = schwab_price_history_params(timeframe)?;
    let start_date = start_time.to_string();
    let end_date = end_time.to_string();
    let response = CLIENT
        .get(format!("{SCHWAB_MARKET_DATA_BASE}/pricehistory"))
        .timeout(SCHWAB_REQUEST_TIMEOUT)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .bearer_auth(access_token.as_str())
        .query(&[
            ("symbol", symbol.as_str()),
            ("periodType", params.period_type),
            ("period", params.period),
            ("frequencyType", params.frequency_type),
            ("frequency", params.frequency),
            ("startDate", start_date.as_str()),
            ("endDate", end_date.as_str()),
            ("needExtendedHoursData", "true"),
            ("needPreviousClose", "false"),
        ])
        .send()
        .await
        .map_err(|e| format!("Schwab price history request failed: {e}"))?;

    let page =
        parse_schwab_json_response::<SchwabPriceHistoryResponse>(response, "Schwab price history")
            .await?;
    let target_interval_ms = timeframe.duration_ms();
    let candles = page
        .candles
        .into_iter()
        .filter_map(|candle| candle.into_candle(params.base_interval_ms))
        .collect::<Vec<_>>();
    Ok(if params.base_interval_ms == target_interval_ms {
        candles
    } else {
        aggregate_candles(candles, target_interval_ms)
    })
}

async fn fetch_schwab_linked_accounts(
    access_token: Zeroizing<String>,
) -> Result<Vec<SchwabLinkedAccount>, String> {
    let response = CLIENT
        .get(format!("{SCHWAB_TRADER_BASE}/accounts/accountNumbers"))
        .timeout(SCHWAB_REQUEST_TIMEOUT)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .bearer_auth(access_token.as_str())
        .send()
        .await
        .map_err(|e| format!("Schwab linked accounts request failed: {e}"))?;

    parse_schwab_json_response::<Vec<SchwabLinkedAccountResponse>>(
        response,
        "Schwab linked accounts",
    )
    .await
    .map(|accounts| {
        accounts
            .into_iter()
            .filter_map(SchwabLinkedAccountResponse::into_linked_account)
            .collect()
    })
}

async fn fetch_schwab_accounts(
    access_token: Zeroizing<String>,
    account_number_to_hash: &HashMap<String, String>,
) -> Result<Vec<SchwabAccountSummary>, String> {
    let response = CLIENT
        .get(format!("{SCHWAB_TRADER_BASE}/accounts"))
        .timeout(SCHWAB_REQUEST_TIMEOUT)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .bearer_auth(access_token.as_str())
        .query(&[("fields", "positions")])
        .send()
        .await
        .map_err(|e| format!("Schwab accounts request failed: {e}"))?;

    let accounts =
        parse_schwab_json_response::<Vec<SchwabAccountEnvelope>>(response, "Schwab accounts")
            .await?;
    Ok(accounts
        .into_iter()
        .filter_map(|account| account.into_summary(account_number_to_hash))
        .collect())
}

async fn parse_schwab_json_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    context: &str,
) -> Result<T, String> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let text = response
        .text()
        .await
        .map_err(|e| format!("{context} response read failed: {e}"))?;
    if !status.is_success() {
        let excerpt =
            redact_sensitive_response_text(&crate::helpers::sensitive_response_excerpt(&text, 240));
        return Err(format!("{context} failed with HTTP {status}: {excerpt}"));
    }
    if let Some(content_type) = content_type.as_deref()
        && !content_type.contains("json")
    {
        let excerpt =
            redact_sensitive_response_text(&crate::helpers::sensitive_response_excerpt(&text, 160));
        return Err(format!("{context} returned non-JSON response: {excerpt}"));
    }
    serde_json::from_str::<T>(&text).map_err(|e| format!("{context} JSON parse failed: {e}"))
}

fn mask_identifier(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "No account".to_string();
    }
    let suffix: String = trimmed
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("...{suffix}")
}

struct SchwabPriceHistoryParams {
    period_type: &'static str,
    period: &'static str,
    frequency_type: &'static str,
    frequency: &'static str,
    base_interval_ms: u64,
}

fn schwab_price_history_params(timeframe: Timeframe) -> Result<SchwabPriceHistoryParams, String> {
    match timeframe {
        Timeframe::M1 => Ok(SchwabPriceHistoryParams {
            period_type: "day",
            period: "10",
            frequency_type: "minute",
            frequency: "1",
            base_interval_ms: Timeframe::M1.duration_ms(),
        }),
        Timeframe::M3 => Ok(SchwabPriceHistoryParams {
            period_type: "day",
            period: "10",
            frequency_type: "minute",
            frequency: "1",
            base_interval_ms: Timeframe::M1.duration_ms(),
        }),
        Timeframe::M5 => Ok(SchwabPriceHistoryParams {
            period_type: "day",
            period: "10",
            frequency_type: "minute",
            frequency: "5",
            base_interval_ms: Timeframe::M5.duration_ms(),
        }),
        Timeframe::M15 => Ok(SchwabPriceHistoryParams {
            period_type: "day",
            period: "10",
            frequency_type: "minute",
            frequency: "15",
            base_interval_ms: Timeframe::M15.duration_ms(),
        }),
        Timeframe::M30 => Ok(SchwabPriceHistoryParams {
            period_type: "day",
            period: "10",
            frequency_type: "minute",
            frequency: "30",
            base_interval_ms: Timeframe::M30.duration_ms(),
        }),
        Timeframe::H1 | Timeframe::H2 | Timeframe::H4 | Timeframe::H8 | Timeframe::H12 => {
            Ok(SchwabPriceHistoryParams {
                period_type: "day",
                period: "10",
                frequency_type: "minute",
                frequency: "30",
                base_interval_ms: Timeframe::M30.duration_ms(),
            })
        }
        Timeframe::D1 | Timeframe::D3 => Ok(SchwabPriceHistoryParams {
            period_type: "year",
            period: "20",
            frequency_type: "daily",
            frequency: "1",
            base_interval_ms: Timeframe::D1.duration_ms(),
        }),
        Timeframe::W1 => Ok(SchwabPriceHistoryParams {
            period_type: "year",
            period: "20",
            frequency_type: "weekly",
            frequency: "1",
            base_interval_ms: Timeframe::W1.duration_ms(),
        }),
        Timeframe::Mo1 => Ok(SchwabPriceHistoryParams {
            period_type: "year",
            period: "20",
            frequency_type: "monthly",
            frequency: "1",
            base_interval_ms: Timeframe::Mo1.duration_ms(),
        }),
        _ => Err(format!(
            "Schwab price history does not support {} candles in Kerosene yet",
            timeframe.label()
        )),
    }
}

fn aggregate_candles(candles: Vec<Candle>, target_interval_ms: u64) -> Vec<Candle> {
    if candles.is_empty() || target_interval_ms == 0 {
        return candles;
    }

    let mut aggregated = Vec::new();
    let mut current_bucket = None;
    let mut current: Option<Candle> = None;

    for candle in candles {
        let bucket = candle.open_time / target_interval_ms * target_interval_ms;
        if current_bucket != Some(bucket) {
            if let Some(done) = current.take() {
                aggregated.push(done);
            }
            current_bucket = Some(bucket);
            current = Some(Candle {
                open_time: bucket,
                close_time: bucket.saturating_add(target_interval_ms).saturating_sub(1),
                open: candle.open,
                high: candle.high,
                low: candle.low,
                close: candle.close,
                volume: candle.volume,
            });
            continue;
        }

        if let Some(active) = current.as_mut() {
            active.high = active.high.max(candle.high);
            active.low = active.low.min(candle.low);
            active.close = candle.close;
            active.volume += candle.volume;
        }
    }

    if let Some(done) = current {
        aggregated.push(done);
    }
    aggregated
}

#[derive(Deserialize)]
struct SchwabTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct SchwabLinkedAccountResponse {
    #[serde(default, rename = "accountNumber")]
    account_number: Option<String>,
    #[serde(default, rename = "hashValue")]
    hash_value: String,
}

impl SchwabLinkedAccountResponse {
    fn into_linked_account(self) -> Option<SchwabLinkedAccount> {
        (!self.hash_value.trim().is_empty()).then(|| SchwabLinkedAccount {
            account_number: self
                .account_number
                .filter(|account| !account.trim().is_empty()),
            hash_value: self.hash_value,
        })
    }
}

#[derive(Deserialize)]
struct SchwabAccountEnvelope {
    #[serde(rename = "securitiesAccount")]
    securities_account: SchwabSecuritiesAccount,
}

impl SchwabAccountEnvelope {
    fn into_summary(
        self,
        account_number_to_hash: &HashMap<String, String>,
    ) -> Option<SchwabAccountSummary> {
        self.securities_account.into_summary(account_number_to_hash)
    }
}

#[derive(Deserialize)]
struct SchwabSecuritiesAccount {
    #[serde(default, rename = "accountNumber")]
    account_number: Option<String>,
    #[serde(default, rename = "type")]
    account_type: Option<String>,
    #[serde(default, rename = "currentBalances")]
    current_balances: Option<SchwabBalances>,
    #[serde(default)]
    positions: Vec<SchwabPosition>,
}

impl SchwabSecuritiesAccount {
    fn into_summary(
        self,
        account_number_to_hash: &HashMap<String, String>,
    ) -> Option<SchwabAccountSummary> {
        let account_hash = self
            .account_number
            .as_ref()
            .and_then(|number| account_number_to_hash.get(number).cloned())?;
        let balances = self.current_balances.unwrap_or_default();
        Some(SchwabAccountSummary {
            account_number: self.account_number,
            account_hash,
            account_type: self.account_type,
            cash_balance: balances
                .cash_balance
                .or(balances.cash_available_for_trading),
            buying_power: balances.buying_power,
            liquidation_value: balances.liquidation_value,
            positions: self
                .positions
                .into_iter()
                .filter_map(SchwabPosition::into_summary)
                .collect(),
        })
    }
}

#[derive(Default, Deserialize)]
struct SchwabBalances {
    #[serde(default, rename = "cashBalance")]
    cash_balance: Option<f64>,
    #[serde(default, rename = "cashAvailableForTrading")]
    cash_available_for_trading: Option<f64>,
    #[serde(default, rename = "buyingPower")]
    buying_power: Option<f64>,
    #[serde(default, rename = "liquidationValue")]
    liquidation_value: Option<f64>,
}

#[derive(Deserialize)]
struct SchwabPosition {
    #[serde(default)]
    instrument: Option<SchwabInstrument>,
    #[serde(default, rename = "longQuantity")]
    long_quantity: Option<f64>,
    #[serde(default, rename = "shortQuantity")]
    short_quantity: Option<f64>,
    #[serde(default, rename = "marketValue")]
    market_value: Option<f64>,
}

impl SchwabPosition {
    fn into_summary(self) -> Option<SchwabPositionSummary> {
        let symbol = self.instrument?.symbol?;
        let quantity =
            self.long_quantity.unwrap_or_default() - self.short_quantity.unwrap_or_default();
        Some(SchwabPositionSummary {
            symbol,
            quantity,
            market_value: self.market_value,
        })
    }
}

#[derive(Deserialize)]
struct SchwabInstrument {
    #[serde(default)]
    symbol: Option<String>,
}

#[derive(Deserialize)]
struct SchwabPriceHistoryResponse {
    #[serde(default)]
    candles: Vec<SchwabPriceCandle>,
}

#[derive(Deserialize)]
struct SchwabPriceCandle {
    datetime: u64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

impl SchwabPriceCandle {
    fn into_candle(self, interval_ms: u64) -> Option<Candle> {
        if !self.open.is_finite()
            || !self.high.is_finite()
            || !self.low.is_finite()
            || !self.close.is_finite()
            || !self.volume.is_finite()
        {
            return None;
        }
        Some(Candle {
            open_time: self.datetime,
            close_time: self.datetime.saturating_add(interval_ms).saturating_sub(1),
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume.max(0.0),
        })
    }
}

#[cfg(test)]
mod tests;
