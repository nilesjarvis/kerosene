use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartInstance, ChartSurfaceId};

use super::super::capture::ChartScreenshotRenderRequest;
use super::super::label::chart_screenshot_label_style;

use iced::Rectangle;

// ---------------------------------------------------------------------------
// Screenshot Render Requests
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn chart_screenshot_render_request(
        &self,
        instance: &ChartInstance,
        surface_id: ChartSurfaceId,
        logical_bounds: Rectangle,
    ) -> ChartScreenshotRenderRequest {
        let theme = self.theme();
        let chart = chart_for_screenshot_export(instance, &self.chart_screenshot_settings);

        ChartScreenshotRenderRequest {
            symbol: instance.symbol_display.clone(),
            timeframe: instance.interval.label().to_string(),
            chart,
            viewport: self
                .chart_surface_viewports
                .get(&surface_id)
                .copied()
                .or(instance.heatmap_viewport),
            label_style: chart_screenshot_label_style(&theme),
            background_color: theme.extended_palette().background.base.color,
            logical_bounds,
            theme,
        }
    }
}

pub(in crate::chart_screenshot) fn chart_for_screenshot_export(
    instance: &ChartInstance,
    settings: &crate::config::ChartScreenshotSettingsConfig,
) -> crate::chart::CandlestickChart {
    let mut chart = instance.chart.snapshot_for_export();
    chart.obscure_position_prices = settings.obscure_position_entry;
    chart.hide_positions_and_orders = settings.hide_positions_and_orders;
    chart
}
