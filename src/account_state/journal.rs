use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::{api, journal};

use iced::Task;

impl TradingTerminal {
    pub(crate) fn load_journal_for_active_account(&mut self, full_history: bool) -> Task<Message> {
        let account_key = self.active_journal_account_key();
        self.journal.switch_active_account(account_key.clone());

        if self.journal.window_id.is_none() {
            return Task::none();
        }

        if self.journal.loading && !full_history {
            return Task::none();
        }

        let Some(address) = self.connected_address.clone() else {
            self.journal.clear_active_account_data();
            self.journal.error = Some("Connect an account before loading the journal.".to_string());
            return Task::none();
        };

        if self.journal.loaded_address.as_deref() != Some(address.as_str()) {
            self.journal
                .clear_active_account_data_for_address(address.clone());
        }

        self.journal.loading = true;
        self.journal.error = None;
        self.journal.warning = None;

        if !full_history
            && !self.journal_active_account_is_ghost()
            && self.journal.raw_fills.is_empty()
            && let Ok(cached) = journal::load_cache(&address)
        {
            self.journal.raw_fills = cached;
            let aggregation =
                journal::aggregate_trades_with_diagnostics(self.journal.raw_fills.clone());
            self.journal.trades = aggregation.trades;
            self.journal.trade_details = aggregation.trade_details;
            self.journal.clear_snapshot_cache();
            self.journal.warning = aggregation.diagnostics.warning_message();
        }

        let request = if full_history {
            api::UserFillsRequest::full_history()
        } else {
            api::UserFillsRequest::since(journal::newest_fill_time(&self.journal.raw_fills))
        };
        self.journal.sync_status = journal::JournalSyncStatus {
            watermark_ms: request.end_time,
            next_start_ms: Some(request.start_time),
            pages_loaded: 0,
            fills_loaded: self.journal.raw_fills.len(),
            pagination_warning: None,
            complete: false,
        };
        let request_id = self.journal.next_sync_request_id();
        let request_account_key = account_key.clone();
        let request_address = address.clone();

        Task::perform(api::fetch_user_fills(address, request), move |result| {
            Message::JournalFillsLoaded {
                request_id,
                account_key: request_account_key.clone().into(),
                address: request_address.clone().into(),
                result: result.into(),
            }
        })
    }
}
