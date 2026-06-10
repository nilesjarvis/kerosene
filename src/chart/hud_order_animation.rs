use super::model::{CandlestickChart, HudFeedEntry, HudOrderAnimation, MarketOrderLoadingOverlay};
use iced::{Point, Size};

// ---------------------------------------------------------------------------
// HUD Order Animation State
// ---------------------------------------------------------------------------

const HUD_ORDER_ANIMATION_STEP: f32 = 0.18;
const MARKET_ORDER_LOADING_STEP: f32 = 0.045;
/// Armed pulse loops roughly every 1.2s at the 40ms animation tick.
const HUD_ARMED_PULSE_STEP: f32 = 0.033;
/// HUD battle feed rows fade out over this lifetime.
pub(crate) const HUD_FEED_TTL_MS: u64 = 5_000;
pub(crate) const HUD_FEED_MAX_ROWS: usize = 3;

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

        if self.hud_armed {
            self.hud_pulse_phase = (self.hud_pulse_phase + HUD_ARMED_PULSE_STEP).fract();
        } else {
            self.hud_pulse_phase = 0.0;
        }
    }

    pub(crate) fn hud_order_animation_active(&self) -> bool {
        self.hud_order_animation.is_some() || !self.pending_market_order_loading.is_empty()
    }

    /// Animation ticks also run while armed so the HUD pulse stays smooth.
    pub(crate) fn hud_animation_tick_needed(&self) -> bool {
        self.hud_order_animation_active() || self.hud_armed
    }

    pub(crate) fn push_hud_feed(&mut self, label: String, is_buy: bool, now_ms: u64) {
        self.hud_feed
            .retain(|entry| now_ms.saturating_sub(entry.added_at_ms) < HUD_FEED_TTL_MS);
        self.hud_feed.push(HudFeedEntry {
            label,
            is_buy,
            added_at_ms: now_ms,
        });
        if self.hud_feed.len() > HUD_FEED_MAX_ROWS {
            let excess = self.hud_feed.len() - HUD_FEED_MAX_ROWS;
            self.hud_feed.drain(0..excess);
        }
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

#[cfg(test)]
mod tests {
    use crate::chart::CandlestickChart;
    use crate::config::ChartCrosshairStyle;

    #[test]
    fn armed_pulse_advances_with_ticks_and_resets_when_safe() {
        let mut chart = CandlestickChart::new(1);
        chart.set_crosshair_style(ChartCrosshairStyle::Hud);
        assert!(!chart.hud_animation_tick_needed());

        chart.set_hud_armed_at(true, 1_000);
        assert!(chart.hud_animation_tick_needed());
        chart.advance_hud_order_animation();
        assert!(chart.hud_pulse_phase > 0.0);

        // Disarm resets the phase immediately: the tick stops with the arm
        // state, so the transition itself must restart the pulse.
        chart.set_hud_armed_at(false, 2_000);
        assert!(!chart.hud_animation_tick_needed());
        assert_eq!(chart.hud_pulse_phase, 0.0);
    }

    #[test]
    fn hud_feed_caps_rows_and_prunes_expired_entries() {
        let mut chart = CandlestickChart::new(1);
        for index in 0..5u64 {
            chart.push_hud_feed(format!("MKT LONG {index}"), true, 1_000 + index);
        }

        assert_eq!(chart.hud_feed.len(), 3);
        assert_eq!(chart.hud_feed[0].label, "MKT LONG 2");

        chart.push_hud_feed("LIMIT SELL 1 @ 10".to_string(), false, 60_000);
        assert_eq!(chart.hud_feed.len(), 1);
        assert_eq!(chart.hud_feed[0].label, "LIMIT SELL 1 @ 10");
    }

    #[test]
    fn hud_feed_clears_when_the_chart_symbol_changes() {
        let mut chart = CandlestickChart::new(1);
        chart.set_symbol_label("BTC".to_string());
        chart.push_hud_feed("MKT LONG 1 @ 100".to_string(), true, 1_000);

        // Same symbol: shots stay on the feed.
        chart.set_symbol_label("BTC".to_string());
        assert_eq!(chart.hud_feed.len(), 1);

        // Feed rows carry no symbol, so switching instruments clears them.
        chart.set_symbol_label("ETH".to_string());
        assert!(chart.hud_feed.is_empty());
    }
}
