use crate::app_state::TradingTerminal;
use iced::{Color, Theme};

impl TradingTerminal {
    pub(super) fn live_watchlist_price_color(
        &self,
        sym_key: &str,
        now_ms: u64,
        theme: &Theme,
    ) -> Color {
        let mut flash_info = None;
        for candidate in self.mid_candidates_for_symbol(sym_key) {
            if let Some(&info) = self.live_watchlist_flashes.get(&candidate) {
                flash_info = Some(info);
                break;
            }
        }
        if let Some((flash_ts, direction)) = flash_info {
            let elapsed = now_ms.saturating_sub(flash_ts);
            if elapsed < 800 {
                let flash_base = if direction > 0 {
                    theme.palette().success
                } else {
                    theme.palette().danger
                };
                let factor = (elapsed as f32 / 800.0).clamp(0.0, 1.0);
                return Color::from_rgba(
                    flash_base.r + (theme.palette().text.r - flash_base.r) * factor,
                    flash_base.g + (theme.palette().text.g - flash_base.g) * factor,
                    flash_base.b + (theme.palette().text.b - flash_base.b) * factor,
                    1.0,
                );
            }
        }
        theme.palette().text
    }
}
