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
