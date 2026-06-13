use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_wallet_label_io(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ExportWalletLabels => {
                let export = self.wallet_labels_export_with_time(Self::now_ms());
                return Task::perform(
                    async move {
                        let json =
                            serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;

                        let path = rfd::AsyncFileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_file_name("kerosene_wallet_labels.json")
                            .save_file()
                            .await;

                        if let Some(path) = path {
                            std::fs::write(path.path(), json).map_err(|e| e.to_string())?;
                            Ok(())
                        } else {
                            Err("Export cancelled".to_string())
                        }
                    },
                    Message::WalletLabelsExported,
                );
            }
            Message::ImportWalletLabels => {
                if self.config_clear_requested || self.config_cleared_this_session {
                    self.push_toast(
                        "Wallet label import is disabled because config persistence is paused until restart.".to_string(),
                        true,
                    );
                    return Task::none();
                }

                return Task::perform(
                    async {
                        let path = rfd::AsyncFileDialog::new()
                            .add_filter("JSON", &["json"])
                            .pick_file()
                            .await;

                        if let Some(path) = path {
                            let content =
                                std::fs::read_to_string(path.path()).map_err(|e| e.to_string())?;
                            let export: config::WalletLabelsExport =
                                serde_json::from_str(&content).map_err(|e| e.to_string())?;
                            Ok(export)
                        } else {
                            Err("Import cancelled".to_string())
                        }
                    },
                    Message::WalletLabelsImported,
                );
            }
            Message::WalletLabelsExported(result) => match result {
                Ok(_) => self.push_toast("Wallet labels exported successfully".to_string(), false),
                Err(e) => {
                    if e != "Export cancelled" {
                        self.push_toast(format!("Wallet label export failed: {}", e), true)
                    }
                }
            },
            Message::WalletLabelsImported(result) => match result {
                Ok(export) => {
                    if self.config_clear_requested || self.config_cleared_this_session {
                        self.push_toast(
                            "Wallet label import was discarded because config persistence is paused until restart.".to_string(),
                            true,
                        );
                        return Task::none();
                    }

                    let labeled_addresses_before = self.labeled_wallet_addresses();
                    match Self::merge_wallet_label_export(&mut self.address_book, export) {
                        Ok(summary) => {
                            let synced_addresses = self.sync_labeled_addresses_to_wallet_tracker();
                            let labels_changed =
                                self.labeled_wallet_addresses() != labeled_addresses_before;
                            if summary.changed() > 0 || !synced_addresses.is_empty() {
                                if labels_changed {
                                    self.refresh_tracked_trades_subscription();
                                }
                                if self.wallet_tracker.open {
                                    for address in synced_addresses {
                                        self.queue_wallet_tracker_core_refresh(address);
                                    }
                                }
                                self.persist_config();
                            }
                            self.push_toast(summary.toast_text(), false);
                        }
                        Err(e) => {
                            self.push_toast(format!("Wallet label import failed: {}", e), true)
                        }
                    }
                }
                Err(e) => {
                    if e != "Import cancelled" {
                        self.push_toast(format!("Wallet label import failed: {}", e), true)
                    }
                }
            },
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AddressBookEntryConfig, WALLET_LABELS_EXPORT_SCHEMA, WalletLabelsExport};

    const TEST_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

    fn wallet_label_export() -> WalletLabelsExport {
        WalletLabelsExport {
            schema: WALLET_LABELS_EXPORT_SCHEMA.to_string(),
            exported_at_ms: 1,
            labels: vec![AddressBookEntryConfig {
                address: TEST_ADDRESS.to_string(),
                label: "Imported".to_string(),
                color: None,
                tags: Vec::new(),
            }],
        }
    }

    #[test]
    fn completed_wallet_label_import_is_discarded_after_config_clear() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_cleared_this_session = true;
        terminal.wallet_tracker.open = true;

        let _task = terminal
            .update_wallet_label_io(Message::WalletLabelsImported(Ok(wallet_label_export())));

        assert!(terminal.address_book.is_empty());
        assert!(terminal.wallet_tracker.tracked_addresses.is_empty());
        assert!(terminal.wallet_tracker.rows.is_empty());
        assert!(terminal.wallet_tracker.core_refresh_queue.is_empty());
        assert!(terminal.config_save_due_at.is_none());
        assert!(
            terminal.toasts.last().is_some_and(
                |toast| toast.is_error && toast.message.contains("import was discarded")
            )
        );
    }

    #[test]
    fn completed_wallet_label_import_is_discarded_while_config_clear_is_pending() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_clear_requested = true;
        terminal.wallet_tracker.open = true;

        let _task = terminal
            .update_wallet_label_io(Message::WalletLabelsImported(Ok(wallet_label_export())));

        assert!(terminal.address_book.is_empty());
        assert!(terminal.wallet_tracker.tracked_addresses.is_empty());
        assert!(terminal.wallet_tracker.rows.is_empty());
        assert!(terminal.wallet_tracker.core_refresh_queue.is_empty());
        assert!(terminal.config_save_due_at.is_none());
        assert!(
            terminal.toasts.last().is_some_and(
                |toast| toast.is_error && toast.message.contains("import was discarded")
            )
        );
    }

    #[test]
    fn wallet_label_import_start_is_blocked_after_config_clear() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_cleared_this_session = true;

        let _task = terminal.update_wallet_label_io(Message::ImportWalletLabels);

        assert!(
            terminal
                .toasts
                .last()
                .is_some_and(|toast| toast.is_error
                    && toast.message.contains("import is disabled"))
        );
    }
}
