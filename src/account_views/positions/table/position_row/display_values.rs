use super::super::sort::PositionRowData;
use super::super::{PositionNumberMode, format_position_display_value};
use super::formatting::format_position_signed_amount;
use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;

// ---------------------------------------------------------------------------
// Display Values
// ---------------------------------------------------------------------------

pub(super) struct PositionRowPnlDisplays {
    pub(super) value: String,
    pub(super) upnl: String,
    pub(super) funding: String,
    pub(super) total: String,
}

impl TradingTerminal {
    pub(super) fn position_row_pnl_displays(
        &self,
        data: &PositionRowData,
        denomination: &DisplayDenominationContext,
        number_mode: PositionNumberMode,
    ) -> PositionRowPnlDisplays {
        let spot_value_unavailable =
            self.is_spot_coin(&data.coin) && data.ap.position.position_value.trim().is_empty();
        let spot_pnl_unavailable =
            self.is_spot_coin(&data.coin) && data.ap.position.unrealized_pnl.trim().is_empty();
        if self.hide_pnl {
            return PositionRowPnlDisplays {
                value: data
                    .position_value
                    .map(|_| self.display_pnl_mask())
                    .unwrap_or_else(|| unavailable_or_invalid(spot_value_unavailable)),
                upnl: data
                    .upnl
                    .map(|_| self.display_pnl_mask())
                    .unwrap_or_else(|| unavailable_or_invalid(spot_pnl_unavailable)),
                funding: "***".to_string(),
                total: data
                    .total_pnl
                    .map(|_| self.display_pnl_mask())
                    .unwrap_or_else(|| unavailable_or_invalid(spot_pnl_unavailable)),
            };
        }

        PositionRowPnlDisplays {
            value: data
                .position_value
                .map(|value| format_position_display_value(denomination, value, number_mode))
                .unwrap_or_else(|| unavailable_or_invalid(spot_value_unavailable)),
            upnl: data
                .upnl
                .map(|upnl| format_position_display_value(denomination, upnl, number_mode))
                .unwrap_or_else(|| unavailable_or_invalid(spot_pnl_unavailable)),
            funding: data
                .funding_since_open
                .map(|funding| format_position_signed_amount(denomination, funding, number_mode))
                .unwrap_or_else(|| "-".to_string()),
            total: data
                .total_pnl
                .map(|total_pnl| {
                    format_position_display_value(denomination, total_pnl, number_mode)
                })
                .unwrap_or_else(|| unavailable_or_invalid(spot_pnl_unavailable)),
        }
    }
}

fn unavailable_or_invalid(unavailable: bool) -> String {
    if unavailable {
        "-".to_string()
    } else {
        "Invalid".to_string()
    }
}
