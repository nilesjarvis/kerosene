use crate::app_state::TradingTerminal;
use crate::message::Message;

use crate::{api, journal};
use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_journal(&mut self, message: Message) -> Task<Message> {
        match message {
            // ----- Trading Journal messages -----
            Message::JournalFillsLoaded {
                account_key,
                address,
                result,
            } => {
                if self.journal.active_account_key != account_key
                    || self.connected_address.as_deref() != Some(address.as_str())
                {
                    return Task::none();
                }

                match result {
                    Ok(page) => {
                        self.journal.loaded_address = Some(address.clone());
                        let fetched_count = page.fills.len();
                        let added = journal::merge_fills(&mut self.journal.raw_fills, page.fills);

                        if let Some(next_request) = page.next_request {
                            let request_account_key = account_key.clone();
                            let request_address = address.clone();
                            return Task::perform(
                                api::fetch_user_fills(address, next_request),
                                move |result| Message::JournalFillsLoaded {
                                    account_key: request_account_key.clone(),
                                    address: request_address.clone(),
                                    result,
                                },
                            );
                        }

                        let aggregation = journal::aggregate_trades_with_diagnostics(
                            self.journal.raw_fills.clone(),
                        );
                        let mut warnings = Vec::new();
                        if let Some(warning) = aggregation.diagnostics.warning_message() {
                            warnings.push(warning);
                        }
                        self.journal.trades = aggregation.trades;
                        self.journal.error = None;

                        if !self.journal_active_account_is_ghost()
                            && let Err(e) = journal::save_cache(&address, &self.journal.raw_fills)
                        {
                            warnings.push(format!("Could not save journal cache: {}", e));
                        }

                        self.journal.loading = false;
                        self.journal.last_refresh_time = Some(page.requested_end_time);

                        if fetched_count == 0 && added == 0 && self.journal.raw_fills.is_empty() {
                            warnings.push("No fills found for this account.".to_string());
                        }

                        self.journal.warning = if warnings.is_empty() {
                            None
                        } else {
                            Some(warnings.join(" "))
                        };
                    }
                    Err(e) => {
                        if self.journal.raw_fills.is_empty() {
                            self.journal.error = Some(e);
                        } else {
                            self.journal.warning = Some(format!(
                                "Journal refresh incomplete: {}. Showing cached data.",
                                e
                            ));
                        }
                        self.journal.loading = false;
                    }
                }
            }
            Message::JournalEditStart(id, source_key) => {
                self.journal.edit_modes.insert(id.clone(), true);
                let note = source_key
                    .as_ref()
                    .and_then(|key| self.journal.entries.get(key))
                    .or_else(|| self.journal.entries.get(&id))
                    .cloned()
                    .unwrap_or_default();
                if let Some(source_key) = source_key {
                    self.journal.edit_source_keys.insert(id.clone(), source_key);
                }
                self.journal.edit_buffers.insert(id, note);
            }
            Message::JournalEditCancel(id) => {
                self.journal.edit_modes.remove(&id);
                self.journal.edit_source_keys.remove(&id);
                self.journal.edit_buffers.remove(&id);
            }
            Message::JournalEditSave(id) => {
                if let Some(note) = self.journal.edit_buffers.remove(&id) {
                    if let Some(source_key) = self.journal.edit_source_keys.remove(&id)
                        && source_key != id
                    {
                        self.journal.entries.remove(&source_key);
                    }
                    if note.open.trim().is_empty() && note.close.trim().is_empty() {
                        self.journal.entries.remove(&id);
                    } else {
                        self.journal.entries.insert(id.clone(), note);
                    }
                    self.persist_config();
                }
                self.journal.edit_modes.remove(&id);
            }
            Message::JournalBufferChanged(id, is_open, text) => {
                let entry = self.journal.edit_buffers.entry(id).or_default();
                if is_open {
                    entry.open = text;
                } else {
                    entry.close = text;
                }
            }
            Message::JournalFilterChanged(filter) => {
                self.journal.filter = filter;
            }
            Message::JournalSortChanged(sort) => {
                self.journal.sort = sort;
            }
            Message::JournalToggleAllAssets => {
                self.journal.show_all_assets = !self.journal.show_all_assets;
            }
            Message::JournalToggleAccountValueChart(show) => {
                self.journal.show_account_value_chart = show;
            }
            Message::JournalRefresh => {
                return self.load_journal_for_active_account(true);
            }
            _ => {}
        }

        Task::none()
    }
}
