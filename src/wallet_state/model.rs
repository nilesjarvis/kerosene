use super::address_book::{AddressBookEntry, normalize_wallet_address_value};
use crate::account::{WalletDetailsData, WalletTrackerSnapshot};
use crate::config::{TrackedWalletConfig, WalletTrackerConfig};

use iced::window;
use std::collections::HashMap;

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

#[derive(Debug, Clone, Default)]
pub(crate) struct WalletTrackerRow {
    pub(crate) loading: bool,
    pub(crate) order_loading: bool,
    pub(crate) snapshot: Option<WalletTrackerSnapshot>,
    pub(crate) last_updated_ms: Option<u64>,
    pub(crate) orders_last_updated_ms: Option<u64>,
    pub(crate) error: Option<String>,
    pub(crate) order_error: Option<String>,
    pub(crate) open_order_count: Option<usize>,
    pub(crate) next_core_retry_ms: Option<u64>,
    pub(crate) next_order_retry_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct WalletDetailsWindowState {
    pub(crate) address: String,
    pub(crate) data: Option<WalletDetailsData>,
    pub(crate) loading: bool,
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
            error: None,
            last_refresh_ms: None,
            width: WALLET_DETAILS_DEFAULT_WIDTH,
            height: WALLET_DETAILS_DEFAULT_HEIGHT,
            x: None,
            y: None,
        }
    }
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
