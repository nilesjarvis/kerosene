use super::annotation_overlays::AnnotationOverlayContext;
use super::candle_layer::CandleLayerContext;
use super::crosshair::CrosshairOverlayContext;
use super::fisheye::ChartFisheye;
use super::model::{CANDLE_GAP_RATIO, CandlestickChart, VOLUME_REGION_RATIO, heatmap_rect_budget};
use super::overlays::TradingOverlayContext;
use super::state::ChartState;
use crate::message::Message;

use iced::mouse;
use iced::widget::canvas;
use iced::{Rectangle, Renderer, Theme};

// ---------------------------------------------------------------------------
// Canvas Program
// ---------------------------------------------------------------------------

pub(super) enum ChartDrawState<'a> {
    Current(&'a ChartState),
    PendingReset(Box<ChartState>),
}

impl ChartDrawState<'_> {
    pub(super) fn as_ref(&self) -> &ChartState {
        match self {
            Self::Current(state) => state,
            Self::PendingReset(state) => state,
        }
    }
}

impl CandlestickChart {
    pub(super) fn chart_state_for_draw<'a>(&self, state: &'a ChartState) -> ChartDrawState<'a> {
        if state.reset_epoch_seen == self.reset_epoch {
            ChartDrawState::Current(state)
        } else {
            ChartDrawState::PendingReset(Box::new(ChartState::reset_for_epoch(self.reset_epoch)))
        }
    }

    pub(crate) fn draw_with_state(
        &self,
        state: &ChartState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        if self.candles.is_empty() {
            return vec![];
        }

        let draw_state = self.chart_state_for_draw(state);
        let state = draw_state.as_ref();

        let chart_w = bounds.width - self.price_axis_width();
        let (chart_h, funding_panel_h, session_panel_h) = self.chart_area_heights(bounds.height);
        if chart_w <= 0.0
            || chart_h <= 0.0
            || !chart_w.is_finite()
            || !chart_h.is_finite()
            || !bounds.width.is_finite()
            || !bounds.height.is_finite()
        {
            return vec![];
        }
        let volume_h = chart_h * VOLUME_REGION_RATIO;
        let price_h = chart_h - volume_h;
        let fisheye = ChartFisheye::new(
            self.fisheye_enabled,
            self.fisheye_strength,
            chart_w,
            chart_h + funding_panel_h + session_panel_h,
        )
        .with_chromatic(
            self.chromatic_aberration_enabled,
            self.chromatic_aberration_strength,
        )
        .with_edge_blur(self.edge_blur_enabled, self.edge_blur_strength);

        let candle_w = state.candle_width;
        let gap = candle_w * CANDLE_GAP_RATIO;
        let step = candle_w + gap;
        let heatmap_rect_budget =
            heatmap_rect_budget(fisheye.distorts_geometry(), state.is_view_panning());

        let Some(visible_range) = self.visible_candle_range(state, chart_w) else {
            return vec![];
        };
        let first_vis = visible_range.first;
        let last_vis = visible_range.last;
        let right_idx = visible_range.right_idx;

        let idx_to_cx = |i: usize| -> f32 {
            let slots_from_right = right_idx - i as isize;
            chart_w - (slots_from_right as f32) * step - step * 0.5
        };

        let Some(price_stats) = self.visible_price_stats_for_state(state, first_vis, last_vis)
        else {
            return vec![];
        };
        let price_lo = price_stats.price_lo;
        let price_hi = price_stats.price_hi;
        let price_range = price_stats.price_range;
        let vol_max = price_stats.volume_max;

        let price_to_y = |price: f64| -> f32 {
            if price_range == 0.0 {
                return price_h * 0.5;
            }
            self.price_to_y_with(price, price_hi, price_range, price_h)
        };
        let candle_bull_color = self.chart_bull_color.unwrap_or(theme.palette().success);
        let candle_bear_color = self.chart_bear_color.unwrap_or(theme.palette().danger);

        let candle_layer_context = CandleLayerContext {
            renderer,
            theme,
            bounds,
            state,
            chart_w,
            chart_h,
            funding_panel_h,
            session_panel_h,
            price_h,
            volume_h,
            candle_w,
            step,
            heatmap_rect_budget,
            first_vis,
            last_vis,
            right_idx,
            price_lo,
            price_hi,
            price_range,
            vol_max,
            candle_bull_color,
            candle_bear_color,
            fisheye,
            idx_to_cx: &idx_to_cx,
            price_to_y: &price_to_y,
        };
        let candles_geo = self.draw_candle_layer(&candle_layer_context);

        let right_axis_badges = self.right_axis_badge_layout(
            state,
            price_h,
            price_range,
            chart_w,
            fisheye,
            &price_to_y,
        );
        let mut overlay_frame = canvas::Frame::new(renderer, bounds.size());

        let chart_region = Rectangle {
            x: 0.0,
            y: 0.0,
            width: bounds.width,
            height: chart_h,
        };
        overlay_frame.with_clip(chart_region, |frame| {
            let mut trading_overlay_context = TradingOverlayContext {
                frame: &mut *frame,
                state,
                theme,
                chart_w,
                price_h,
                price_range,
                candles: &self.candles,
                first_vis,
                last_vis,
                candle_bull_color,
                candle_bear_color,
                right_axis_badges: &right_axis_badges,
                fisheye,
                price_to_y: &price_to_y,
                idx_to_cx: &idx_to_cx,
            };
            self.draw_trading_overlays(&mut trading_overlay_context);

            let mut annotation_overlay_context = AnnotationOverlayContext {
                frame: &mut *frame,
                state,
                theme,
                chart_w,
                chart_h,
                price_h,
                price_range,
                right_axis_badges: &right_axis_badges,
                fisheye,
                price_to_y: &price_to_y,
            };
            self.draw_annotation_overlays(&mut annotation_overlay_context);
        });

        let mut crosshair_context = CrosshairOverlayContext {
            frame: &mut overlay_frame,
            state,
            theme,
            chart_w,
            chart_h,
            funding_panel_h,
            session_panel_h,
            price_h,
            price_hi,
            price_range,
            step,
            fisheye,
            price_to_y: &price_to_y,
        };
        self.draw_crosshair_overlay(&mut crosshair_context);
        self.draw_next_candle_countdown(
            &mut overlay_frame,
            theme,
            chart_w,
            chart_h + funding_panel_h + session_panel_h,
            bounds,
            state.cursor_position,
        );
        let overlay_geo = overlay_frame.into_geometry();

        vec![candles_geo, overlay_geo]
    }
}

impl canvas::Program<Message> for CandlestickChart {
    type State = ChartState;

    fn update(
        &self,
        state: &mut ChartState,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        self.update_interaction(state, event, bounds, cursor)
    }

    fn draw(
        &self,
        state: &ChartState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        self.draw_with_state(state, renderer, theme, bounds, cursor)
    }

    fn mouse_interaction(
        &self,
        state: &ChartState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        self.mouse_interaction_for(state, bounds, cursor)
    }
}
