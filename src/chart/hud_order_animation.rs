use super::model::{
    CandlestickChart, HudFeedEntry, HudOrderAnimation, HudSelectorKind, HudWeaponSelector,
    MarketOrderLoadingOverlay,
};
use iced::{Point, Size};

// ---------------------------------------------------------------------------
// HUD Order Animation State
// ---------------------------------------------------------------------------

const HUD_ORDER_ANIMATION_STEP: f32 = 0.18;
const MARKET_ORDER_LOADING_STEP: f32 = 0.045;
/// Armed pulse loops roughly every 1.2s at the 40ms animation tick.
const HUD_ARMED_PULSE_STEP: f32 = 0.033;
/// Weapon selector lives ~1.6s at the 40ms animation tick.
const HUD_SELECTOR_AGE_STEP: f32 = 0.025;
const HUD_SELECTOR_POP_STEP: f32 = 0.12;
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

        if let Some(selector) = self.hud_weapon_selector.as_mut() {
            selector.age += HUD_SELECTOR_AGE_STEP;
            selector.pop = (selector.pop + HUD_SELECTOR_POP_STEP).min(1.0);
            if selector.age >= 1.0 {
                self.hud_weapon_selector = None;
            }
        }
    }

    /// Pops the weapon-selector list open (or keeps it open) after a
    /// mode/side keypress; a real change restarts the slot highlight pop.
    pub(crate) fn open_hud_weapon_selector(&mut self, kind: HudSelectorKind, changed: bool) {
        let pop = match self.hud_weapon_selector {
            Some(selector) if selector.kind == kind && !changed => selector.pop,
            _ => 0.0,
        };
        self.hud_weapon_selector = Some(HudWeaponSelector {
            kind,
            age: 0.0,
            pop,
        });
    }

    pub(crate) fn hud_order_animation_active(&self) -> bool {
        self.hud_order_animation.is_some() || !self.pending_market_order_loading.is_empty()
    }

    /// Animation ticks also run while armed (for the pulse) and while the
    /// weapon selector is open (for its pop and fade).
    pub(crate) fn hud_animation_tick_needed(&self) -> bool {
        self.hud_order_animation_active() || self.hud_armed || self.hud_weapon_selector.is_some()
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
    fn weapon_selector_pops_open_ticks_and_expires() {
        use crate::chart::model::HudSelectorKind;

        let mut chart = CandlestickChart::new(1);
        chart.set_crosshair_style(ChartCrosshairStyle::Hud);
        assert!(!chart.hud_animation_tick_needed());

        chart.open_hud_weapon_selector(HudSelectorKind::Mode, true);
        assert!(chart.hud_animation_tick_needed());
        let selector = chart.hud_weapon_selector.expect("selector open");
        assert_eq!(selector.kind, HudSelectorKind::Mode);
        assert_eq!(selector.pop, 0.0);

        chart.advance_hud_order_animation();
        let selector = chart.hud_weapon_selector.expect("selector still open");
        assert!(selector.age > 0.0);
        assert!(selector.pop > 0.0);

        // A repeat press of the equipped slot keeps the list open without
        // restarting the highlight pop; a real change restarts it.
        let pop_before = selector.pop;
        chart.open_hud_weapon_selector(HudSelectorKind::Mode, false);
        let selector = chart.hud_weapon_selector.expect("selector refreshed");
        assert_eq!(selector.age, 0.0);
        assert_eq!(selector.pop, pop_before);
        chart.open_hud_weapon_selector(HudSelectorKind::Side, true);
        let selector = chart.hud_weapon_selector.expect("selector switched");
        assert_eq!(selector.kind, HudSelectorKind::Side);
        assert_eq!(selector.pop, 0.0);

        for _ in 0..41 {
            chart.advance_hud_order_animation();
        }
        assert_eq!(chart.hud_weapon_selector, None);
        assert!(!chart.hud_animation_tick_needed());
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
