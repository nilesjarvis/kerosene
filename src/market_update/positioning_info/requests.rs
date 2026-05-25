use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::positioning_state::{
    POSITIONING_INFO_LIMIT, POSITIONING_INFO_OFFSET, PositioningInfoId, PositioningInfoPage,
};

use iced::Task;

mod apply;
mod planning;
mod queue;

use planning::{PositioningInfoChangeRequestPlan, PositioningInfoRequestPlan};

// ---------------------------------------------------------------------------
// Request Lifecycle
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn request_positioning_info_refresh_all(&mut self, force: bool) -> Task<Message> {
        let ids: Vec<PositioningInfoId> = self
            .panes
            .iter()
            .filter_map(|(_, kind)| {
                if let PaneKind::PositioningInfo(id) = kind {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        Task::batch(
            ids.into_iter()
                .map(|id| self.request_positioning_info_refresh(id, force)),
        )
    }

    pub(crate) fn request_positioning_info_refresh(
        &mut self,
        id: PositioningInfoId,
        force: bool,
    ) -> Task<Message> {
        if self
            .positioning_infos
            .get(&id)
            .is_some_and(|instance| instance.page == PositioningInfoPage::Change)
        {
            return self.request_positioning_info_change_refresh(id, force);
        }
        self.request_positioning_info_positions_refresh(id, force)
    }

    pub(super) fn request_positioning_info_positions_refresh(
        &mut self,
        id: PositioningInfoId,
        force: bool,
    ) -> Task<Message> {
        let Some(plan) = self.positioning_info_request_plan(id, force) else {
            return Task::none();
        };

        match plan {
            PositioningInfoRequestPlan::Fetch {
                request_key,
                coin,
                side,
                sort_field,
                sort_order,
            } => self.queue_positioning_info_fetch(
                id,
                request_key,
                coin,
                side,
                sort_field,
                sort_order,
            ),
            PositioningInfoRequestPlan::Status(message, is_error) => {
                if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    instance.loading = false;
                    instance.pending_key = None;
                    instance.error = Some(message.clone());
                    if is_error {
                        instance.data = None;
                    }
                }
                if is_error && force {
                    self.push_toast(message, true);
                }
                Task::none()
            }
        }
    }

    pub(super) fn request_positioning_info_change_refresh(
        &mut self,
        id: PositioningInfoId,
        force: bool,
    ) -> Task<Message> {
        let Some(plan) = self.positioning_info_change_request_plan(id, force) else {
            return Task::none();
        };

        match plan {
            PositioningInfoChangeRequestPlan::Fetch {
                request_key,
                market,
                timeframe,
            } => self.queue_positioning_info_change_fetch(id, request_key, market, timeframe),
            PositioningInfoChangeRequestPlan::Status(message, is_error) => {
                if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    instance.change_loading = false;
                    instance.change_pending_key = None;
                    instance.change_error = Some(message.clone());
                    if is_error {
                        instance.change_data = None;
                    }
                }
                if is_error && force {
                    self.push_toast(message, true);
                }
                Task::none()
            }
        }
    }
}

pub(super) fn positioning_info_request_key(
    coin: &str,
    side: &str,
    sort_field: &str,
    sort_order: &str,
) -> String {
    format!(
        "{coin}:{side}:{sort_field}:{sort_order}:{}:{}",
        POSITIONING_INFO_LIMIT, POSITIONING_INFO_OFFSET
    )
}

pub(super) fn positioning_info_change_request_key(market: &str, timeframe: &str) -> String {
    format!("change:{market}:{timeframe}")
}
