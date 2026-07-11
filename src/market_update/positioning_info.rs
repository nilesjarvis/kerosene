use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::positioning_state::PositioningInfoId;

use iced::Task;

mod requests;
mod symbols;

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
                    instance.symbol_picker_open = true;
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
            Message::PositioningInfoEntryMinChanged(id, value) => {
                if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    instance.entry_min_input = value;
                }
                Task::none()
            }
            Message::PositioningInfoEntryMaxChanged(id, value) => {
                if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    instance.entry_max_input = value;
                }
                Task::none()
            }
            Message::ApplyPositioningInfoEntryRange(id) => {
                if let Some(instance) = self.positioning_infos.get_mut(&id) {
                    instance.error = None;
                    instance.data = None;
                    instance.pending_key = None;
                } else {
                    return Task::none();
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
            Message::PositioningInfoWsAssetCtxUpdate(symbol, source_context, ctx) => {
                if !self.market_stream_source_is_current(source_context) {
                    return Task::none();
                }
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
            Message::PositioningInfoWsAssetCtxLagged(symbol, source_context, _skipped) => {
                if !self.market_stream_source_is_current(source_context) {
                    return Task::none();
                }
                if self.symbol_key_is_hidden(&symbol) {
                    return Task::none();
                }
                for instance in self
                    .positioning_infos
                    .values_mut()
                    .filter(|instance| instance.symbol == symbol)
                {
                    instance.asset_ctx = None;
                    instance.asset_ctx_updated_at_ms = None;
                }
                Task::none()
            }
            Message::PositioningInfoLoaded(request_key, generation, result) => {
                self.apply_positioning_info_loaded(request_key, generation, result.into_result())
            }
            Message::PositioningInfoChangeLoaded(request_key, generation, result) => self
                .apply_positioning_info_change_loaded(
                    request_key,
                    generation,
                    result.into_result(),
                ),
            _ => Task::none(),
        }
    }
}
