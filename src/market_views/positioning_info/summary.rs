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
use iced::{Alignment, Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Positioning Information Summaries
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_positioning_info_summary(
        &self,
        data: &TickerPositions,
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
        row![
            view_positioning_summary_metric(
                "Net",
                format_signed_usd_compact(net_notional, &denomination),
                net_color,
                theme,
            ),
            view_positioning_summary_metric(
                "Traders",
                format_decimal_with_commas(data.total_count as f64, 0),
                theme.palette().text,
                theme,
            ),
            view_positioning_summary_metric("Rows", rows_label, theme.palette().text, theme),
        ]
        .spacing(16)
        .width(Fill)
        .into()
    }

    pub(super) fn view_positioning_info_change_summary(
        &self,
        data: &PerpDeltas,
        instance: &PositioningInfoInstance,
        theme: &Theme,
    ) -> Element<'static, Message> {
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
            text(format!("API: {}", data.timeframe))
                .size(10)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .wrap()
        .into()
    }
}

fn view_positioning_summary_metric(
    label: &'static str,
    value: String,
    value_color: Color,
    theme: &Theme,
) -> Element<'static, Message> {
    column![
        text(label)
            .size(10)
            .color(theme.extended_palette().background.weak.text),
        text(value)
            .size(12)
            .font(crate::app_fonts::monospace_font())
            .color(value_color)
            .width(Fill),
    ]
    .spacing(4)
    .width(Fill)
    .into()
}
