use super::model::CandlestickChart;
use super::state::ChartState;
use iced::widget::canvas;
use iced::{Color, Theme};

mod current_price;
mod orders;
mod positions;

// ---------------------------------------------------------------------------
// Trading Overlays
// ---------------------------------------------------------------------------

pub(super) struct TradingOverlayContext<'a, PriceToY>
where
    PriceToY: Fn(f64) -> f32,
{
    pub(super) frame: &'a mut canvas::Frame,
    pub(super) state: &'a ChartState,
    pub(super) theme: &'a Theme,
    pub(super) chart_w: f32,
    pub(super) price_h: f32,
    pub(super) price_range: f64,
    pub(super) candle_bull_color: Color,
    pub(super) candle_bear_color: Color,
    pub(super) price_to_y: &'a PriceToY,
}

impl CandlestickChart {
    pub(super) fn draw_trading_overlays<PriceToY>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY>,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        self.draw_current_price_line(ctx);
        self.draw_active_position_lines(ctx);
        self.draw_active_order_lines(ctx);
    }
}
