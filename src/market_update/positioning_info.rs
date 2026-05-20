use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::hyperdash_api::{
    PerpDeltas, TickerPositions, fetch_perp_deltas, fetch_ticker_positions,
};
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::positioning_state::{
    POSITIONING_INFO_LIMIT, POSITIONING_INFO_OFFSET, PositioningInfoId, PositioningInfoInstance,
    PositioningInfoPage,
};
use iced::Task;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn positioning_symbol_search_input_id(id: PositioningInfoId) -> iced::widget::Id {
        iced::widget::Id::from(format!("positioning_symbol_search_{id}"))
    }

    pub(crate) fn focus_positioning_symbol_search_input(id: PositioningInfoId) -> Task<Message> {
        iced::widget::operation::focus(Self::positioning_symbol_search_input_id(id))
    }

    pub(crate) fn update_positioning_info_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddPositioningInfoPane => self.add_positioning_info_pane(),
            Message::PositioningInfoPageChanged(id, page) => {
                if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    if instance.page == page {
                        return Task::none();
                    }
                    instance.page = page;
                    instance.symbol_picker_open = false;
                }
                self.persist_config();
                self.request_positioning_info_refresh(id, false)
            }
            Message::PositioningInfoSearchChanged(id, query) => {
                if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    instance.search_query = query;
                    if instance.page == PositioningInfoPage::Change {
                        instance.symbol_picker_open = true;
                    }
                }
                Task::none()
            }
            Message::TogglePositioningInfoSymbolPicker(id) => {
                let opened = if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    instance.symbol_picker_open = !instance.symbol_picker_open;
                    if instance.symbol_picker_open {
                        instance.search_query.clear();
                    }
                    instance.symbol_picker_open
                } else {
                    false
                };
                if opened {
                    Self::focus_positioning_symbol_search_input(id)
                } else {
                    Task::none()
                }
            }
            Message::PositioningInfoSymbolSelected(id, symbol) => {
                self.select_positioning_info_symbol(id, symbol)
            }
            Message::PositioningInfoSideChanged(id, side) => {
                if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    if instance.side == side {
                        return Task::none();
                    }
                    instance.side = side;
                    instance.error = None;
                    instance.data = None;
                }
                self.persist_config();
                self.request_positioning_info_positions_refresh(id, true)
            }
            Message::PositioningInfoSortChanged(id, sort_field) => {
                if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    if instance.sort_field == sort_field {
                        instance.sort_direction = match instance.sort_direction {
                            crate::config::SortDirection::Ascending => {
                                crate::config::SortDirection::Descending
                            }
                            crate::config::SortDirection::Descending => {
                                crate::config::SortDirection::Ascending
                            }
                        };
                    } else {
                        instance.sort_field = sort_field;
                        instance.sort_direction = sort_field.default_direction();
                    }
                    instance.error = None;
                    instance.data = None;
                }
                self.persist_config();
                self.request_positioning_info_positions_refresh(id, true)
            }
            Message::PositioningInfoChangeTimeframeChanged(id, timeframe) => {
                if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    if instance.change_timeframe == timeframe {
                        return Task::none();
                    }
                    instance.change_timeframe = timeframe;
                    instance.change_loading = false;
                    instance.change_error = None;
                    instance.change_data = None;
                    instance.change_pending_key = None;
                }
                self.persist_config();
                self.request_positioning_info_change_refresh(id, true)
            }
            Message::PositioningInfoChangeSortChanged(id, sort_field) => {
                if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    if instance.change_sort_field == sort_field {
                        instance.change_sort_direction = match instance.change_sort_direction {
                            crate::config::SortDirection::Ascending => {
                                crate::config::SortDirection::Descending
                            }
                            crate::config::SortDirection::Descending => {
                                crate::config::SortDirection::Ascending
                            }
                        };
                    } else {
                        instance.change_sort_field = sort_field;
                        instance.change_sort_direction = sort_field.default_direction();
                    }
                }
                self.persist_config();
                Task::none()
            }
            Message::ClearPositioningInfoFilters(id) => {
                let should_refresh = if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    let should_refresh = instance.has_active_filters() || instance.error.is_some();
                    instance.reset_filters();
                    instance.error = None;
                    instance.data = None;
                    instance.pending_key = None;
                    should_refresh
                } else {
                    false
                };
                if should_refresh {
                    self.persist_config();
                    self.request_positioning_info_positions_refresh(id, true)
                } else {
                    Task::none()
                }
            }
            Message::RefreshPositioningInfoPane(id) => {
                self.request_positioning_info_refresh(id, true)
            }
            Message::RefreshPositioningInfo => self.request_positioning_info_refresh_all(false),
            Message::PositioningInfoWsAssetCtxUpdate(symbol, ctx) => {
                if self.symbol_key_is_hidden(&symbol) {
                    return Task::none();
                }
                let now_ms = Self::now_ms();
                for instance in self
                    .positioning_infos
                    .values_mut()
                    .filter(|instance| instance.symbol == symbol)
                {
                    instance.asset_ctx = Some(ctx.clone());
                    instance.asset_ctx_updated_at_ms = Some(now_ms);
                }
                Task::none()
            }
            Message::PositioningInfoLoaded(request_key, result) => {
                self.apply_positioning_info_loaded(request_key, *result)
            }
            Message::PositioningInfoChangeLoaded(request_key, result) => {
                self.apply_positioning_info_change_loaded(request_key, *result)
            }
            _ => Task::none(),
        }
    }

    fn add_positioning_info_pane(&mut self) -> Task<Message> {
        self.add_widget_menu_open = false;
        let Some(focus) = self.add_target_pane() else {
            self.push_toast(
                "Could not add Positioning Information: no pane is available".to_string(),
                true,
            );
            return Task::none();
        };

        let id = self.next_positioning_info_id;
        self.next_positioning_info_id = self.next_positioning_info_id.saturating_add(1);
        let symbol = self.visible_positioning_symbol(&self.active_symbol);
        self.positioning_infos
            .insert(id, PositioningInfoInstance::new(id, symbol));

        if self
            .add_pane_to_target(
                self.add_widget_axis(),
                focus,
                PaneKind::PositioningInfo(id),
                "Positioning Information",
            )
            .is_none()
        {
            self.positioning_infos.remove(&id);
            return Task::none();
        }

        self.request_positioning_info_refresh(id, true)
    }

    fn select_positioning_info_symbol(
        &mut self,
        id: PositioningInfoId,
        symbol: String,
    ) -> Task<Message> {
        if self.symbol_key_is_hidden(&symbol) {
            if let Some(instance) = self.positioning_infos.get_mut(&id) {
                let error = "Ticker is hidden in Settings > Risk".to_string();
                instance.symbol_picker_open = false;
                instance.error = Some(error.clone());
                instance.change_error = Some(error);
                instance.asset_ctx = None;
                instance.asset_ctx_updated_at_ms = None;
            }
            return Task::none();
        }
        if self
            .exchange_symbols
            .iter()
            .find(|candidate| candidate.key == symbol)
            .is_some_and(|candidate| candidate.market_type != MarketType::Perp)
        {
            if let Some(instance) = self.positioning_infos.get_mut(&id) {
                let error =
                    "Positioning Information is only available for perp symbols".to_string();
                instance.symbol_picker_open = false;
                instance.error = Some(error.clone());
                instance.change_error = Some(error);
                instance.asset_ctx = None;
                instance.asset_ctx_updated_at_ms = None;
            }
            return Task::none();
        }

        if let Some(instance) = self.positioning_infos.get_mut(&id) {
            if instance.symbol == symbol {
                instance.search_query.clear();
                instance.symbol_picker_open = false;
                return Task::none();
            }
            instance.symbol = symbol;
            instance.search_query.clear();
            instance.symbol_picker_open = false;
            instance.loading = false;
            instance.error = None;
            instance.data = None;
            instance.asset_ctx = None;
            instance.asset_ctx_updated_at_ms = None;
            instance.change_loading = false;
            instance.change_error = None;
            instance.change_data = None;
            instance.change_pending_key = None;
            instance.pending_key = None;
        }
        self.persist_config();
        self.request_positioning_info_refresh(id, true)
    }

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

    pub(crate) fn request_positioning_info_positions_refresh(
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

    pub(crate) fn request_positioning_info_change_refresh(
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

    fn positioning_info_request_plan(
        &self,
        id: PositioningInfoId,
        force: bool,
    ) -> Option<PositioningInfoRequestPlan> {
        let instance = self.positioning_infos.get(&id)?;
        if instance.loading && !force {
            return None;
        }
        if self.hyperdash_api_key.trim().is_empty() {
            return Some(PositioningInfoRequestPlan::Status(
                "Add HyperDash key in Settings > Integrations".to_string(),
                true,
            ));
        }
        if instance.symbol.trim().is_empty() {
            return Some(PositioningInfoRequestPlan::Status(
                "Select a ticker".to_string(),
                false,
            ));
        }
        if self.symbol_key_is_hidden(&instance.symbol) {
            return Some(PositioningInfoRequestPlan::Status(
                "Ticker is hidden in Settings > Risk".to_string(),
                true,
            ));
        }
        let Some(coin) = self.hyperdash_coin_for_symbol(&instance.symbol) else {
            return Some(PositioningInfoRequestPlan::Status(
                "Positioning Information is only available for perp symbols".to_string(),
                false,
            ));
        };

        let side = instance.side.api_value().to_string();
        let sort_field = instance.sort_field.api_field().to_string();
        let sort_order = positioning_info_sort_order(instance.sort_direction).to_string();
        let request_key = positioning_info_request_key(&coin, &side, &sort_field, &sort_order);
        Some(PositioningInfoRequestPlan::Fetch {
            request_key,
            coin,
            side,
            sort_field,
            sort_order,
        })
    }

    fn queue_positioning_info_fetch(
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

        let api_key = self.hyperdash_api_key.trim().to_string();
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
            move |result| Message::PositioningInfoLoaded(request_key.clone(), Box::new(result)),
        )
    }

    fn positioning_info_change_request_plan(
        &self,
        id: PositioningInfoId,
        force: bool,
    ) -> Option<PositioningInfoChangeRequestPlan> {
        let instance = self.positioning_infos.get(&id)?;
        if instance.change_loading && !force {
            return None;
        }
        if self.hyperdash_api_key.trim().is_empty() {
            return Some(PositioningInfoChangeRequestPlan::Status(
                "Add HyperDash key in Settings > Integrations".to_string(),
                true,
            ));
        }
        if instance.symbol.trim().is_empty() {
            return Some(PositioningInfoChangeRequestPlan::Status(
                "Select a ticker".to_string(),
                false,
            ));
        }
        if self.symbol_key_is_hidden(&instance.symbol) {
            return Some(PositioningInfoChangeRequestPlan::Status(
                "Ticker is hidden in Settings > Risk".to_string(),
                true,
            ));
        }
        let Some(market) = self.hyperdash_coin_for_symbol(&instance.symbol) else {
            return Some(PositioningInfoChangeRequestPlan::Status(
                "Positioning Information is only available for perp symbols".to_string(),
                false,
            ));
        };

        let timeframe = instance.change_timeframe.api_value().to_string();
        let request_key = positioning_info_change_request_key(&market, &timeframe);
        Some(PositioningInfoChangeRequestPlan::Fetch {
            request_key,
            market,
            timeframe,
        })
    }

    fn queue_positioning_info_change_fetch(
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

        let api_key = self.hyperdash_api_key.trim().to_string();
        Task::perform(
            fetch_perp_deltas(market, timeframe, api_key),
            move |result| {
                Message::PositioningInfoChangeLoaded(request_key.clone(), Box::new(result))
            },
        )
    }

    fn apply_positioning_info_loaded(
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

    fn apply_positioning_info_change_loaded(
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

    pub(crate) fn visible_positioning_symbol(&self, candidate: &str) -> String {
        let candidate = candidate.trim();
        if !candidate.is_empty()
            && !self.symbol_key_is_hidden(candidate)
            && self.hyperdash_coin_for_symbol(candidate).is_some()
        {
            return candidate.to_string();
        }
        if !self.active_symbol.is_empty()
            && !self.symbol_key_is_hidden(&self.active_symbol)
            && self
                .hyperdash_coin_for_symbol(&self.active_symbol)
                .is_some()
        {
            return self.active_symbol.clone();
        }
        self.fallback_unmuted_symbol_key()
            .filter(|symbol| self.hyperdash_coin_for_symbol(symbol).is_some())
            .unwrap_or_else(|| "HYPE".to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PositioningInfoRequestPlan {
    Fetch {
        request_key: String,
        coin: String,
        side: String,
        sort_field: String,
        sort_order: String,
    },
    Status(String, bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PositioningInfoChangeRequestPlan {
    Fetch {
        request_key: String,
        market: String,
        timeframe: String,
    },
    Status(String, bool),
}

fn positioning_info_request_key(
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

fn positioning_info_change_request_key(market: &str, timeframe: &str) -> String {
    format!("change:{market}:{timeframe}")
}

fn positioning_info_sort_order(direction: crate::config::SortDirection) -> &'static str {
    match direction {
        crate::config::SortDirection::Ascending => "asc",
        crate::config::SortDirection::Descending => "desc",
    }
}
