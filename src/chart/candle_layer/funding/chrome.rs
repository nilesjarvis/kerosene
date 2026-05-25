use super::super::CandleLayerContext;
use crate::chart::model::CandlestickChart;

use iced::widget::canvas;
use iced::{Color, Point, Size};

mod button;
mod labels;

// ---------------------------------------------------------------------------
// Funding Panel Chrome
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_funding_panel_chrome<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        for offset in [-8.0_f32, 0.0, 8.0] {
            frame.fill_rectangle(
                Point::new(ctx.chart_w * 0.5 + offset - 2.5, panel_y + 2.0),
                Size::new(5.0, 1.0),
                Color {
                    a: 0.28,
                    ..ctx.theme.palette().text
                },
            );
        }
        self.draw_funding_mode_button(ctx, frame, panel_y);
    }
}
