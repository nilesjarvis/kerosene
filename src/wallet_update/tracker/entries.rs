use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::wallet_state::WalletTrackerRow;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_wallet_tracker_entries(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WalletTrackerInputChanged(value) => {
                self.wallet_tracker.add_input = value.into_string();
            }
            Message::WalletTrackerLabelInputChanged(value) => {
                self.wallet_tracker.add_label_input = value;
            }
            Message::WalletTrackerAdd => {
                let Some(addr) = Self::normalize_wallet_address(&self.wallet_tracker.add_input)
                else {
                    self.push_toast("Invalid wallet address".to_string(), true);
                    return Task::none();
                };
                let was_muted = self.wallet_tracker.unmute_address(&addr);
                if self.wallet_tracker.tracked_addresses.contains(&addr) {
                    if was_muted {
                        let label = self.wallet_tracker.add_label_input.trim();
                        if !label.is_empty() {
                            self.address_book.entry(addr.clone()).or_default().label =
                                label.to_string();
                        }
                        self.refresh_tracked_trades_subscription();
                        self.wallet_tracker.rows.entry(addr.clone()).or_default();
                        self.wallet_tracker.add_input.clear();
                        self.wallet_tracker.add_label_input.clear();
                        self.persist_config();
                        self.queue_wallet_tracker_core_refresh(addr);
                        return self.refresh_next_wallet_tracker_core();
                    }
                    self.push_toast("Wallet already shown in tracker".to_string(), true);
                    return Task::none();
                }

                self.wallet_tracker.tracked_addresses.push(addr.clone());
                let label = self.wallet_tracker.add_label_input.trim();
                if !label.is_empty() {
                    self.address_book.entry(addr.clone()).or_default().label = label.to_string();
                    self.refresh_tracked_trades_subscription();
                }
                self.wallet_tracker.rows.insert(
                    addr.clone(),
                    WalletTrackerRow {
                        loading: false,
                        ..Default::default()
                    },
                );
                self.wallet_tracker.add_input.clear();
                self.wallet_tracker.add_label_input.clear();
                self.persist_config();
                self.queue_wallet_tracker_core_refresh(addr);
                return self.refresh_next_wallet_tracker_core();
            }
            Message::WalletTrackerMute(address) => {
                let address = address.into_string();
                let normalized_address =
                    Self::normalize_wallet_address(&address).unwrap_or(address);
                if !self
                    .wallet_tracker
                    .tracked_addresses
                    .contains(&normalized_address)
                {
                    return Task::none();
                }
                if self.wallet_tracker.mute_address(&normalized_address) {
                    self.refresh_tracked_trades_subscription();
                    self.persist_config();
                }
            }
            Message::WalletTrackerUnmute(address) => {
                let address = address.into_string();
                let normalized_address =
                    Self::normalize_wallet_address(&address).unwrap_or(address);
                if self.wallet_tracker.unmute_address(&normalized_address) {
                    self.refresh_tracked_trades_subscription();
                    self.persist_config();
                }
            }
            Message::WalletTrackerRemove(address) => {
                let address = address.into_string();
                let normalized_address =
                    Self::normalize_wallet_address(&address).unwrap_or(address);
                let was_labeled = self.wallet_label(&normalized_address).is_some();
                self.wallet_tracker
                    .tracked_addresses
                    .retain(|tracked| tracked != &normalized_address);
                self.wallet_tracker.unmute_address(&normalized_address);
                self.wallet_tracker.rows.remove(&normalized_address);
                self.wallet_tracker
                    .core_refresh_queue
                    .retain(|queued| queued != &normalized_address);
                self.wallet_tracker
                    .order_refresh_queue
                    .retain(|queued| queued != &normalized_address);
                self.address_book.remove(&normalized_address);
                if was_labeled {
                    self.refresh_tracked_trades_subscription();
                }
                self.persist_config();
            }
            Message::WalletTrackerLabelChanged(address, label) => {
                let Some(address) = Self::normalize_wallet_address(address.as_str()) else {
                    return Task::none();
                };
                let was_labeled = self.wallet_label(&address).is_some();
                let label = label.trim().to_string();
                if label.is_empty() {
                    if self
                        .address_book
                        .get(&address)
                        .is_some_and(|entry| entry.color.is_none() && entry.tags.is_empty())
                    {
                        self.address_book.remove(&address);
                    } else if let Some(entry) = self.address_book.get_mut(&address) {
                        entry.label.clear();
                    }
                } else {
                    self.address_book.entry(address.clone()).or_default().label = label;
                }
                let is_labeled = self.wallet_label(&address).is_some();
                if was_labeled != is_labeled {
                    self.refresh_tracked_trades_subscription();
                }
                self.persist_config();
            }
            _ => {}
        }

        Task::none()
    }
}
