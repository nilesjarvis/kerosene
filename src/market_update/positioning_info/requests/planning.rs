use super::{positioning_info_change_request_key, positioning_info_request_key};
use crate::app_state::TradingTerminal;
use crate::config::SortDirection;
use crate::positioning_state::PositioningInfoId;

// ---------------------------------------------------------------------------
// Request Planning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PositioningInfoRequestPlan {
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
pub(super) enum PositioningInfoChangeRequestPlan {
    Fetch {
        request_key: String,
        market: String,
        timeframe: String,
    },
    Status(String, bool),
}

impl TradingTerminal {
    pub(super) fn positioning_info_request_plan(
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

    pub(super) fn positioning_info_change_request_plan(
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
}

fn positioning_info_sort_order(direction: SortDirection) -> &'static str {
    match direction {
        SortDirection::Ascending => "asc",
        SortDirection::Descending => "desc",
    }
}
