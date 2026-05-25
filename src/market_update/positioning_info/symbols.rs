use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::positioning_state::{PositioningInfoId, PositioningInfoInstance};

use iced::Task;

// ---------------------------------------------------------------------------
// Pane And Symbol Selection
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn add_positioning_info_pane(&mut self) -> Task<Message> {
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

    pub(super) fn select_positioning_info_symbol(
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
