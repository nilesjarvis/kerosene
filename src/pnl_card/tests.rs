use super::*;
use crate::denomination::DisplayDenominationContext;

use iced::{Color, Theme};

mod calculations;
mod contrast;
mod export;
mod metrics;
mod privacy;
mod render_text;
mod state;

fn sample_metrics() -> PnlCardMetrics {
    PnlCardMetrics {
        ticker: "BTC".to_string(),
        leverage_display: "20x".to_string(),
        entry_display: "82,543.2".to_string(),
        exit_display: "84,612.8".to_string(),
        context: "Short 0.52 BTC".to_string(),
        private_context: Some("Short position".to_string()),
        upnl: 1076.19,
        asset_move_pct: Some(2.51),
        leveraged_pct: Some(50.14),
    }
}

fn test_account() -> String {
    "0x1111111111111111111111111111111111111111".to_string()
}

fn other_account() -> String {
    "0x2222222222222222222222222222222222222222".to_string()
}

fn position_state(ticker: &str) -> PnlCardWindowState {
    PnlCardWindowState::new(PnlCardTarget::Position(ticker.to_string()), test_account())
}

fn summary_state() -> PnlCardWindowState {
    PnlCardWindowState::new(PnlCardTarget::Summary, test_account())
}

fn render_test_image(
    state: &PnlCardWindowState,
    metrics: PnlCardMetrics,
    pnl_color: Color,
) -> PnlCardImage {
    match render_pnl_card_image(
        state,
        metrics,
        DisplayDenominationContext::default(),
        pnl_color,
        &Theme::Dark,
    ) {
        Ok(image) => image,
        Err(e) => panic!("pnl card image should render: {e}"),
    }
}
