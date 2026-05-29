use super::model::{CandlestickChart, HudOrderAnimation, MarketOrderLoadingOverlay};
use iced::{Point, Size};

// ---------------------------------------------------------------------------
// HUD Order Animation State
// ---------------------------------------------------------------------------

const HUD_ORDER_ANIMATION_STEP: f32 = 0.18;
const MARKET_ORDER_LOADING_STEP: f32 = 0.045;

impl CandlestickChart {
    pub(crate) fn start_hud_order_animation(
        &mut self,
        price: f64,
        origin: Point,
        chart_size: Size,
        is_buy: bool,
        show_line: bool,
    ) {
        if !price.is_finite() || chart_size.width <= 0.0 || chart_size.height <= 0.0 {
            return;
        }

        self.hud_order_animation = Some(HudOrderAnimation {
            price,
            origin_x: origin.x.clamp(0.0, chart_size.width),
            click_y: origin.y.clamp(0.0, chart_size.height),
            chart_w: chart_size.width,
            chart_h: chart_size.height,
            is_buy,
            show_line,
            progress: 0.0,
        });
    }

    pub(crate) fn advance_hud_order_animation(&mut self) {
        if let Some(animation) = self.hud_order_animation.as_mut() {
            animation.progress += HUD_ORDER_ANIMATION_STEP;
            if animation.progress >= 1.0 {
                self.hud_order_animation = None;
            }
        }

        for loader in &mut self.pending_market_order_loading {
            loader.progress = (loader.progress + MARKET_ORDER_LOADING_STEP).fract();
        }
    }

    pub(crate) fn hud_order_animation_active(&self) -> bool {
        self.hud_order_animation.is_some() || !self.pending_market_order_loading.is_empty()
    }

    pub(crate) fn set_pending_market_order_loading<I>(&mut self, pending: I)
    where
        I: IntoIterator<Item = (u64, bool)>,
    {
        let previous = std::mem::take(&mut self.pending_market_order_loading);
        self.pending_market_order_loading = pending
            .into_iter()
            .map(|(pending_id, is_buy)| {
                let progress = previous
                    .iter()
                    .find(|loader| loader.pending_id == pending_id)
                    .map(|loader| loader.progress)
                    .unwrap_or(0.0);
                MarketOrderLoadingOverlay {
                    pending_id,
                    is_buy,
                    progress,
                }
            })
            .collect();
    }
}
