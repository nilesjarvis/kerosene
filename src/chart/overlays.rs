use super::model::CandlestickChart;
use super::price_badges::RightAxisBadgeLayout;
use super::state::ChartState;
use crate::api::Candle;
use iced::widget::canvas;
use iced::{Color, Theme};

mod current_price;
mod orders;
mod positions;
mod quick_order;
mod trades;

// ---------------------------------------------------------------------------
// Trading Overlays
// ---------------------------------------------------------------------------

pub(super) struct TradingOverlayContext<'a, PriceToY, IdxToCx>
where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    pub(super) frame: &'a mut canvas::Frame,
    pub(super) state: &'a ChartState,
    pub(super) theme: &'a Theme,
    pub(super) chart_w: f32,
    pub(super) price_h: f32,
    pub(super) price_range: f64,
    pub(super) candles: &'a [Candle],
    pub(super) first_vis: usize,
    pub(super) last_vis: usize,
    pub(super) candle_bull_color: Color,
    pub(super) candle_bear_color: Color,
    pub(super) right_axis_badges: &'a RightAxisBadgeLayout,
    pub(super) price_to_y: &'a PriceToY,
    pub(super) idx_to_cx: &'a IdxToCx,
}

impl CandlestickChart {
    pub(super) fn draw_trading_overlays<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        self.draw_current_price_line(ctx);
        self.draw_quick_order_limit_line(ctx);
        if self.should_draw_position_and_order_overlays() {
            self.draw_active_position_lines(ctx);
            self.draw_active_order_lines(ctx);
        }
        self.draw_trade_markers(ctx);
    }

    fn should_draw_position_and_order_overlays(&self) -> bool {
        !self.hide_positions_and_orders
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_and_order_overlay_render_guard_follows_privacy_flag() {
        let mut chart = CandlestickChart::new(1);
        assert!(chart.should_draw_position_and_order_overlays());

        chart.hide_positions_and_orders = true;
        assert!(!chart.should_draw_position_and_order_overlays());
    }
}
