use super::metrics::*;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::helpers::format_decimal_with_commas;
use crate::hyperdash_api::{PerpDeltas, TickerPositions};
use crate::message::Message;
use crate::positioning_state::{
    POSITIONING_CHANGE_ROW_LIMIT, POSITIONING_INFO_LIMIT, PositioningInfoInstance,
};

use iced::widget::{column, row, text};
use iced::{Alignment, Element, Theme};

// ---------------------------------------------------------------------------
// Positioning Information Summaries
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_positioning_info_summary(
        &self,
        data: &TickerPositions,
        instance: &PositioningInfoInstance,
        now_ms: u64,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let net_notional = data.total_long_notional - data.total_short_notional;
        let net_color = signed_value_color(net_notional, theme);
        let denomination = self.display_denomination_context();
        let rows_label = if data.has_more {
            format!("Top {} of {}", POSITIONING_INFO_LIMIT, data.total_count)
        } else {
            data.positions.len().to_string()
        };
        let updated = format_positioning_timestamp(&data.timestamp);
        let last_fetch = instance
            .last_fetch_ms
            .map(|last| format!("{} ago", helpers::format_relative_time(last, now_ms)))
            .unwrap_or_else(|| "-".to_string());

        column![
            row![
                helpers::label_value(
                    "Long",
                    format_usd_number(data.total_long_notional, &denomination)
                ),
                helpers::label_value(
                    "Short",
                    format_usd_number(data.total_short_notional, &denomination)
                ),
                helpers::label_value_colored(
                    "Net",
                    format_signed_usd(net_notional, &denomination),
                    net_color
                ),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            row![
                helpers::label_value(
                    "Traders",
                    format_decimal_with_commas(data.total_count as f64, 0),
                ),
                helpers::label_value("Rows", rows_label),
                helpers::label_value("Updated", updated),
                helpers::label_value("Fetched", last_fetch),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ]
        .spacing(4)
        .into()
    }

    pub(super) fn view_positioning_info_change_summary(
        &self,
        data: &PerpDeltas,
        instance: &PositioningInfoInstance,
        now_ms: u64,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let last_fetch = instance
            .change_last_fetch_ms
            .map(|last| format!("{} ago", helpers::format_relative_time(last, now_ms)))
            .unwrap_or_else(|| "-".to_string());
        let shown = data.deltas.len().min(POSITIONING_CHANGE_ROW_LIMIT);
        let rows_label = if shown < data.deltas.len() {
            format!("Showing {shown} of {}", data.deltas.len())
        } else {
            data.deltas.len().to_string()
        };
        let totals = positioning_change_side_delta_totals(&data.deltas);
        let long_delta_color = positioning_side_delta_color(totals.long_delta, true, theme);
        let short_delta_color = positioning_side_delta_color(totals.short_delta, false, theme);

        row![
            helpers::label_value("Timeframe", instance.change_timeframe.label().to_string()),
            helpers::label_value("Rows", rows_label),
            helpers::label_value_colored(
                "\u{0394} Long",
                format_signed_size(totals.long_delta, true),
                long_delta_color
            ),
            helpers::label_value_colored(
                "\u{0394} Short",
                format_signed_size(totals.short_delta, true),
                short_delta_color
            ),
            helpers::label_value("Fetched", last_fetch),
            text(format!("API: {}", data.timeframe))
                .size(10)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
    }
}
