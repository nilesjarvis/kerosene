use crate::app_state::TradingTerminal;
use crate::hyperdash_api::{PerpDeltas, TickerPositions};
use crate::message::Message;

use iced::Task;

// ---------------------------------------------------------------------------
// Request Result Application
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::market_update::positioning_info) fn apply_positioning_info_loaded(
        &mut self,
        request_key: String,
        result: Result<TickerPositions, String>,
    ) -> Task<Message> {
        let pending = self
            .positioning_info_pending
            .remove(&request_key)
            .unwrap_or_default();
        for id in pending {
            let Some(instance) = self.positioning_infos.get_mut(&id) else {
                continue;
            };
            if instance.pending_key.as_deref() != Some(request_key.as_str()) {
                continue;
            }
            instance.loading = false;
            instance.pending_key = None;
            match &result {
                Ok(data) => {
                    instance.data = Some(data.clone());
                    instance.error = None;
                    instance.last_fetch_ms = Some(Self::now_ms());
                }
                Err(error) => {
                    instance.error = Some(error.clone());
                }
            }
        }
        Task::none()
    }

    pub(in crate::market_update::positioning_info) fn apply_positioning_info_change_loaded(
        &mut self,
        request_key: String,
        result: Result<PerpDeltas, String>,
    ) -> Task<Message> {
        let pending = self
            .positioning_info_pending
            .remove(&request_key)
            .unwrap_or_default();
        for id in pending {
            let Some(instance) = self.positioning_infos.get_mut(&id) else {
                continue;
            };
            if instance.change_pending_key.as_deref() != Some(request_key.as_str()) {
                continue;
            }
            instance.change_loading = false;
            instance.change_pending_key = None;
            match &result {
                Ok(data) => {
                    instance.change_data = Some(data.clone());
                    instance.change_error = None;
                    instance.change_last_fetch_ms = Some(Self::now_ms());
                }
                Err(error) => {
                    instance.change_error = Some(error.clone());
                }
            }
        }
        Task::none()
    }
}
