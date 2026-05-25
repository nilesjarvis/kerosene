use crate::denomination::DisplayDenominationContext;
pub(super) use crate::helpers::positive_percent_change as percent_change;
use iced::{Color, Theme};

use super::{
    TICKER_TAPE_ICON_SIZE, TICKER_TAPE_ITEM_HORIZONTAL_PADDING, TICKER_TAPE_ITEM_MAX_WIDTH,
    TICKER_TAPE_ITEM_MIN_WIDTH, TICKER_TAPE_ITEM_SPACING, TICKER_TAPE_TEXT_CHAR_WIDTH,
};

// ---------------------------------------------------------------------------
// Ticker Tape Formatting
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct TickerTapeItem {
    pub(super) symbol: String,
    pub(super) ticker: String,
    pub(super) price: Option<f64>,
    pub(super) pct_24h: Option<f64>,
}

pub(super) fn ticker_tape_item_width(
    item: &TickerTapeItem,
    denomination: &DisplayDenominationContext,
) -> f32 {
    let text_chars = item.ticker.chars().count()
        + price_label(item.price, denomination).chars().count()
        + percent_label(item.pct_24h).chars().count();
    let text_width = text_chars as f32 * TICKER_TAPE_TEXT_CHAR_WIDTH;
    let padding = f32::from(TICKER_TAPE_ITEM_HORIZONTAL_PADDING) * 2.0;
    let spacing = TICKER_TAPE_ITEM_SPACING as f32 * 3.0;
    let width = TICKER_TAPE_ICON_SIZE + text_width + padding + spacing;

    width
        .ceil()
        .clamp(TICKER_TAPE_ITEM_MIN_WIDTH, TICKER_TAPE_ITEM_MAX_WIDTH)
}

pub(super) fn price_label(price: Option<f64>, denomination: &DisplayDenominationContext) -> String {
    price
        .map(|price| denomination.format_price(price))
        .unwrap_or_else(|| "-".to_string())
}

pub(super) fn percent_label(pct: Option<f64>) -> String {
    pct.map(|pct| format!("{pct:+.2}%"))
        .unwrap_or_else(|| "-".to_string())
}

pub(super) fn pct_color(pct: Option<f64>, theme: &Theme) -> Color {
    match pct {
        Some(value) if value >= 0.0 => theme.palette().success,
        Some(_) => theme.palette().danger,
        None => theme.extended_palette().background.weak.text,
    }
}
