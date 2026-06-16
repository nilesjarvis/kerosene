use crate::app_state::TradingTerminal;
use crate::hyperdash_api::{fetch_perp_deltas, fetch_ticker_positions};
use crate::message::Message;
use crate::positioning_state::{
    POSITIONING_INFO_LIMIT, POSITIONING_INFO_OFFSET, PositioningInfoId,
};

use iced::Task;

// ---------------------------------------------------------------------------
// Request Queueing
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn queue_positioning_info_fetch(
        &mut self,
        id: PositioningInfoId,
        request_key: String,
        coin: String,
        side: String,
        sort_field: String,
        sort_order: String,
    ) -> Task<Message> {
        if let Some(waiting) = self.positioning_info_pending.get_mut(&request_key) {
            if !waiting.contains(&id) {
                waiting.push(id);
            }
            if let Some(instance) = self.positioning_infos.get_mut(&id) {
                instance.loading = true;
                instance.error = None;
                instance.pending_key = Some(request_key);
            }
            return Task::none();
        }

        self.positioning_info_pending
            .insert(request_key.clone(), vec![id]);
        if let Some(instance) = self.positioning_infos.get_mut(&id) {
            instance.loading = true;
            instance.error = None;
            instance.pending_key = Some(request_key.clone());
        }

        let hyperdash_generation = self.hyperdash_key_generation;
        let api_key = self.hyperdash_api_key_for_task();
        Task::perform(
            fetch_ticker_positions(
                coin,
                POSITIONING_INFO_LIMIT,
                POSITIONING_INFO_OFFSET,
                side,
                sort_field,
                sort_order,
                api_key,
            ),
            move |result| {
                Message::PositioningInfoLoaded(
                    request_key.clone(),
                    hyperdash_generation,
                    Box::new(result),
                )
            },
        )
    }

    pub(super) fn queue_positioning_info_change_fetch(
        &mut self,
        id: PositioningInfoId,
        request_key: String,
        market: String,
        timeframe: String,
    ) -> Task<Message> {
        if let Some(waiting) = self.positioning_info_pending.get_mut(&request_key) {
            if !waiting.contains(&id) {
                waiting.push(id);
            }
            if let Some(instance) = self.positioning_infos.get_mut(&id) {
                instance.change_loading = true;
                instance.change_error = None;
                instance.change_pending_key = Some(request_key);
            }
            return Task::none();
        }

        self.positioning_info_pending
            .insert(request_key.clone(), vec![id]);
        if let Some(instance) = self.positioning_infos.get_mut(&id) {
            instance.change_loading = true;
            instance.change_error = None;
            instance.change_pending_key = Some(request_key.clone());
        }

        let hyperdash_generation = self.hyperdash_key_generation;
        let api_key = self.hyperdash_api_key_for_task();
        Task::perform(
            fetch_perp_deltas(market, timeframe, api_key),
            move |result| {
                Message::PositioningInfoChangeLoaded(
                    request_key.clone(),
                    hyperdash_generation,
                    Box::new(result),
                )
            },
        )
    }
}
