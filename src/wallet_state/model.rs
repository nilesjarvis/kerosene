use super::address_book::{AddressBookEntry, normalize_wallet_address_value};
use crate::account::{WalletDetailsData, WalletTrackerSnapshot};
use crate::config::{TrackedWalletConfig, WalletTrackerConfig};
use crate::read_data_provider::ReadDataRequestContext;

use iced::window;
use std::{collections::HashMap, fmt};

pub(crate) const WALLET_TRACKER_CORE_TICK_SECS: u64 = 5;
pub(crate) const WALLET_TRACKER_CORE_MIN_AGE_MS: u64 = 60_000;
pub(crate) const WALLET_TRACKER_CORE_ERROR_BACKOFF_MS: u64 = 60_000;
pub(crate) const WALLET_TRACKER_ORDER_TICK_SECS: u64 = 60;
pub(crate) const WALLET_TRACKER_ORDER_MIN_AGE_MS: u64 = 10 * 60_000;
pub(crate) const WALLET_TRACKER_ORDER_ERROR_BACKOFF_MS: u64 = 5 * 60_000;
pub(crate) const WALLET_DETAILS_DEFAULT_WIDTH: f32 = 980.0;
pub(crate) const WALLET_DETAILS_DEFAULT_HEIGHT: f32 = 640.0;

// ---------------------------------------------------------------------------
// Wallet tracker state
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub(crate) struct WalletTrackerRow {
    pub(crate) loading: bool,
    pub(crate) order_loading: bool,
    pub(crate) loading_context: Option<ReadDataRequestContext>,
    pub(crate) order_loading_context: Option<ReadDataRequestContext>,
    pub(crate) snapshot: Option<WalletTrackerSnapshot>,
    pub(crate) last_updated_ms: Option<u64>,
    pub(crate) orders_last_updated_ms: Option<u64>,
    pub(crate) error: Option<String>,
    pub(crate) order_error: Option<String>,
    pub(crate) open_order_count: Option<usize>,
    pub(crate) next_core_retry_ms: Option<u64>,
    pub(crate) next_order_retry_ms: Option<u64>,
}

impl fmt::Debug for WalletTrackerRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletTrackerRow")
            .field("loading", &self.loading)
            .field("order_loading", &self.order_loading)
            .field("loading_context", &self.loading_context)
            .field("order_loading_context", &self.order_loading_context)
            .field("snapshot", &redacted_presence(&self.snapshot))
            .field("last_updated_ms", &self.last_updated_ms)
            .field("orders_last_updated_ms", &self.orders_last_updated_ms)
            .field("error", &redacted_presence(&self.error))
            .field("order_error", &redacted_presence(&self.order_error))
            .field("open_order_count", &self.open_order_count)
            .field("next_core_retry_ms", &self.next_core_retry_ms)
            .field("next_order_retry_ms", &self.next_order_retry_ms)
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct WalletDetailsWindowState {
    pub(crate) address: String,
    pub(crate) data: Option<WalletDetailsData>,
    pub(crate) loading: bool,
    pub(crate) loading_context: Option<ReadDataRequestContext>,
    pub(crate) error: Option<String>,
    pub(crate) last_refresh_ms: Option<u64>,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) x: Option<f32>,
    pub(crate) y: Option<f32>,
}

impl WalletDetailsWindowState {
    pub(crate) fn new(address: String) -> Self {
        Self {
            address,
            data: None,
            loading: true,
            loading_context: None,
            error: None,
            last_refresh_ms: None,
            width: WALLET_DETAILS_DEFAULT_WIDTH,
            height: WALLET_DETAILS_DEFAULT_HEIGHT,
            x: None,
            y: None,
        }
    }
}

impl fmt::Debug for WalletDetailsWindowState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletDetailsWindowState")
            .field("address", &"<redacted>")
            .field("data", &redacted_presence(&self.data))
            .field("loading", &self.loading)
            .field("loading_context", &self.loading_context)
            .field("error", &redacted_presence(&self.error))
            .field("last_refresh_ms", &self.last_refresh_ms)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("x", &self.x)
            .field("y", &self.y)
            .finish()
    }
}

fn redacted_presence<T>(value: &Option<T>) -> Option<&'static str> {
    value.as_ref().map(|_| "<redacted>")
}

pub(crate) struct WalletTrackerState {
    pub(crate) window_id: Option<window::Id>,
    pub(crate) open: bool,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) x: Option<f32>,
    pub(crate) y: Option<f32>,
    pub(crate) add_input: String,
    pub(crate) add_label_input: String,
    pub(crate) tracked_addresses: Vec<String>,
    pub(crate) muted_addresses: Vec<String>,
    pub(crate) rows: HashMap<String, WalletTrackerRow>,
    pub(crate) core_refresh_queue: Vec<String>,
    pub(crate) order_refresh_queue: Vec<String>,
}

impl WalletTrackerState {
    pub(crate) fn from_config(cfg: &WalletTrackerConfig) -> Self {
        let mut tracked_addresses = Vec::new();
        for address in &cfg.tracked_addresses {
            if let Some(address) = normalize_wallet_address_value(address)
                && !tracked_addresses.contains(&address)
            {
                tracked_addresses.push(address);
            }
        }
        for wallet in &cfg.wallets {
            if let Some(address) = normalize_wallet_address_value(&wallet.address)
                && !tracked_addresses.contains(&address)
            {
                tracked_addresses.push(address);
            }
        }
        let mut muted_addresses = Vec::new();
        for address in &cfg.muted_addresses {
            if let Some(address) = normalize_wallet_address_value(address)
                && !muted_addresses.contains(&address)
            {
                muted_addresses.push(address);
            }
        }

        Self {
            window_id: None,
            open: cfg.open,
            width: cfg.width,
            height: cfg.height,
            x: cfg.x,
            y: cfg.y,
            add_input: String::new(),
            add_label_input: String::new(),
            tracked_addresses,
            muted_addresses,
            rows: HashMap::new(),
            core_refresh_queue: Vec::new(),
            order_refresh_queue: Vec::new(),
        }
    }

    pub(crate) fn is_muted(&self, address: &str) -> bool {
        self.muted_addresses.iter().any(|muted| muted == address)
    }

    pub(crate) fn mute_address(&mut self, address: &str) -> bool {
        if self.is_muted(address) {
            return false;
        }
        self.muted_addresses.push(address.to_string());
        true
    }

    pub(crate) fn unmute_address(&mut self, address: &str) -> bool {
        let original_len = self.muted_addresses.len();
        self.muted_addresses.retain(|muted| muted != address);
        self.muted_addresses.len() != original_len
    }

    pub(crate) fn to_config(
        &self,
        address_book: &HashMap<String, AddressBookEntry>,
    ) -> WalletTrackerConfig {
        let wallets = self
            .tracked_addresses
            .iter()
            .map(|address| TrackedWalletConfig {
                address: address.clone(),
                label: address_book
                    .get(address)
                    .map(|entry| entry.label.trim().to_string())
                    .unwrap_or_default(),
            })
            .collect();

        WalletTrackerConfig {
            tracked_addresses: self.tracked_addresses.clone(),
            muted_addresses: self.muted_addresses.clone(),
            wallets,
            open: self.open,
            width: self.width,
            height: self.height,
            x: self.x,
            y: self.y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WalletDetailsWindowState, WalletTrackerRow};
    use crate::account::{
        ClearinghouseState, MarginSummary, SpotClearinghouseState, WalletDetailsData,
        WalletTrackerSnapshot,
    };

    const TEST_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

    #[test]
    fn wallet_details_window_state_debug_redacts_address() {
        let state = WalletDetailsWindowState::new(TEST_ADDRESS.to_string());

        let rendered = format!("{state:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains(TEST_ADDRESS));
    }

    #[test]
    fn wallet_details_window_state_debug_redacts_data_and_error() {
        let mut state = WalletDetailsWindowState::new(TEST_ADDRESS.to_string());
        state.data = Some(WalletDetailsData {
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "wallet-detail-secret-equity".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
                asset_positions: Vec::new(),
            },
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            positions: Vec::new(),
            open_orders: Vec::new(),
            fills: Vec::new(),
            warnings: vec!["wallet-detail-secret-warning".to_string()],
            fetched_at_ms: 42,
        });
        state.error = Some("wallet-detail-secret-error".to_string());

        let rendered = format!("{state:?}");

        assert!(rendered.contains("data: Some(\"<redacted>\")"));
        assert!(rendered.contains("error: Some(\"<redacted>\")"));
        assert!(!rendered.contains("wallet-detail-secret-equity"));
        assert!(!rendered.contains("wallet-detail-secret-warning"));
        assert!(!rendered.contains("wallet-detail-secret-error"));
    }

    #[test]
    fn wallet_tracker_row_debug_redacts_snapshot_and_errors() {
        let row = WalletTrackerRow {
            snapshot: Some(WalletTrackerSnapshot {
                equity: Some(987654321.123),
                withdrawable: Some(123.0),
                unrealized_pnl: Some(-456.0),
                margin_used_pct: Some(7.89),
                open_trade_count: Some(3),
                open_order_count: 5,
                long_exposure: Some(1000.0),
                short_exposure: Some(2000.0),
                valuation_warning: Some("wallet-row-secret-valuation-warning".to_string()),
            }),
            error: Some("wallet-row-secret-error".to_string()),
            order_error: Some("wallet-row-secret-order-error".to_string()),
            ..Default::default()
        };

        let rendered = format!("{row:?}");

        assert!(rendered.contains("snapshot: Some(\"<redacted>\")"));
        assert!(rendered.contains("error: Some(\"<redacted>\")"));
        assert!(rendered.contains("order_error: Some(\"<redacted>\")"));
        assert!(!rendered.contains("987654321.123"));
        assert!(!rendered.contains("wallet-row-secret-error"));
        assert!(!rendered.contains("wallet-row-secret-order-error"));
        assert!(!rendered.contains("wallet-row-secret-valuation-warning"));
    }
}
