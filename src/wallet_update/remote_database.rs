use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::wallet_state::remote_database::{
    RemoteWalletDatabaseResult, RemoteWalletDatabaseState, RemoteWalletSnapshot, api,
};
use iced::Task;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

// Never reuse an ID across config resets or endpoint switches.
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

impl TradingTerminal {
    pub(super) fn update_remote_wallet_database(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RemoteWalletDatabaseUrlChanged(value) => {
                self.wallet_tracker.remote_database.input = value.into_zeroizing().to_string();
                self.wallet_tracker.remote_database.input_error = None;
            }
            Message::DisconnectRemoteWalletDatabase => {
                self.wallet_tracker.remote_database.input.clear();
                return self.update_remote_wallet_database(Message::SaveRemoteWalletDatabase);
            }
            Message::SaveRemoteWalletDatabase => {
                let url = match api::normalize_base_url(&self.wallet_tracker.remote_database.input)
                {
                    Ok(url) => url,
                    Err(error) => {
                        self.wallet_tracker.remote_database.input_error = Some(error);
                        return Task::none();
                    }
                };
                if self.wallet_tracker.remote_database.url != url {
                    self.replace_remote_wallet_snapshot(RemoteWalletSnapshot::default());
                    self.wallet_tracker.remote_database = RemoteWalletDatabaseState {
                        input: url.clone(),
                        url,
                        ..Default::default()
                    };
                    self.persist_config();
                } else {
                    self.wallet_tracker.remote_database.input = url;
                    self.wallet_tracker.remote_database.input_error = None;
                }
                return self.request_remote_wallet_sync();
            }
            Message::RemoteWalletDatabaseSync => return self.request_remote_wallet_sync(),
            Message::RemoteWalletDatabaseLoaded(request_id, result) => {
                if self.wallet_tracker.remote_database.pending_request != Some(request_id) {
                    return Task::none();
                }
                self.wallet_tracker.remote_database.pending_request = None;
                match result.0 {
                    Ok(snapshot) => {
                        self.replace_remote_wallet_snapshot(snapshot);
                        self.wallet_tracker.remote_database.last_synced_ms = Some(Self::now_ms());
                        self.wallet_tracker.remote_database.error = None;
                        if self.wallet_tracker_is_visible() {
                            return self.refresh_next_wallet_tracker_core();
                        }
                    }
                    Err(error) => self.wallet_tracker.remote_database.error = Some(error),
                }
            }
            _ => {}
        }
        Task::none()
    }

    pub(crate) fn request_remote_wallet_sync(&mut self) -> Task<Message> {
        let remote = &mut self.wallet_tracker.remote_database;
        if remote.url.is_empty() || remote.pending_request.is_some() {
            return Task::none();
        }
        let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        remote.pending_request = Some(request_id);
        Task::perform(api::fetch_wallets(remote.url.clone()), move |result| {
            Message::RemoteWalletDatabaseLoaded(request_id, RemoteWalletDatabaseResult(result))
        })
    }

    fn replace_remote_wallet_snapshot(&mut self, snapshot: RemoteWalletSnapshot) {
        let previous_subscriptions = self.tracked_trade_subscription_addresses();
        let tracker = &mut self.wallet_tracker;
        let remote = &mut tracker.remote_database;
        // A local import of an address during this session gives it local ownership.
        remote
            .added_addresses
            .retain(|address| !self.address_book.contains_key(address));
        let removed: HashSet<_> = remote
            .added_addresses
            .iter()
            .filter(|address| !snapshot.entries.contains_key(*address))
            .cloned()
            .collect();
        tracker
            .tracked_addresses
            .retain(|address| !removed.contains(address));
        tracker
            .muted_addresses
            .retain(|address| !removed.contains(address));
        tracker.rows.retain(|address, _| !removed.contains(address));
        tracker
            .core_refresh_queue
            .retain(|address| !removed.contains(address));
        tracker
            .order_refresh_queue
            .retain(|address| !removed.contains(address));
        tracker
            .compact_selections
            .retain(|_, selection| !removed.contains(selection.address.as_str()));
        remote
            .added_addresses
            .retain(|address| !removed.contains(address));
        let tracked: HashSet<_> = tracker.tracked_addresses.iter().collect();
        let mut added: Vec<_> = snapshot
            .entries
            .keys()
            .filter(|address| !tracked.contains(address))
            .cloned()
            .collect();
        added.sort();
        for address in added {
            tracker.tracked_addresses.push(address.clone());
            remote.added_addresses.insert(address.clone());
            tracker.rows.entry(address.clone()).or_default();
            tracker.core_refresh_queue.push(address);
        }
        remote.entries = snapshot.entries;
        remote.skipped = snapshot.skipped;
        if previous_subscriptions != self.tracked_trade_subscription_addresses() {
            self.refresh_tracked_trades_subscription();
        }
    }
}

#[cfg(test)]
mod tests;
