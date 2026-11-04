use crate::app_state::TradingTerminal;
use crate::denomination::format_compact;
use crate::feed_views::summary::FEED_SUMMARY_COLUMN_SPACING;
use crate::feed_views::{FEED_SUMMARY_TIMEFRAMES, view_feed_summary};
use crate::message::Message;
use iced::Element;

// Room for both side totals at 10px, including full USD amounts in the billions.
const MIN_SUMMARY_COLUMN_WIDTH: f32 = 180.0;

fn visible_summary_timeframes(available_width: f32) -> &'static [(u64, &'static str)] {
    let columns = ((available_width + FEED_SUMMARY_COLUMN_SPACING)
        / (MIN_SUMMARY_COLUMN_WIDTH + FEED_SUMMARY_COLUMN_SPACING)) as usize;
    let columns = columns.clamp(1, FEED_SUMMARY_TIMEFRAMES.len());
    // Remove the oldest, then newest, working inward while preserving order:
    // 1m/5m/15m/1H -> 1m/5m/15m -> 5m/15m -> 5m.
    let start = (FEED_SUMMARY_TIMEFRAMES.len() - columns) / 2;
    &FEED_SUMMARY_TIMEFRAMES[start..start + columns]
}

impl TradingTerminal {
    pub(super) fn view_liquidations_summary(
        &self,
        now_ms: u64,
        available_width: f32,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let (buy_color, sell_color) = self.direction_colors(&theme);
        let denomination = self.display_denomination_context();
        view_feed_summary(
            &theme,
            visible_summary_timeframes(available_width),
            [("L", sell_color), ("S", buy_color)],
            |minutes| self.calculate_liquidation_summary(minutes, now_ms),
            |value| {
                if available_width < MIN_SUMMARY_COLUMN_WIDTH
                    && let Some(value) = denomination.convert_usd_value(value)
                {
                    denomination.format_active_amount("", format_compact(value))
                } else {
                    denomination.format_value(value, 0)
                }
            },
        )
    }
}

#[cfg(test)]
mod tests;
