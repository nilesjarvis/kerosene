use crate::denomination::DisplayDenominationContext;
use crate::denomination::format_compact_usd;
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

#[derive(Debug, Clone)]
pub(super) struct PreparedTickerTapeItem {
    pub(super) symbol: String,
    pub(super) ticker: String,
    pub(super) price_label: String,
    pub(super) percent_label: String,
    pub(super) pct_24h: Option<f64>,
    pub(super) width: f32,
}

impl TickerTapeItem {
    pub(super) fn prepare(
        self,
        denomination: &DisplayDenominationContext,
    ) -> PreparedTickerTapeItem {
        let price_label = price_label(self.price, denomination);
        let percent_label = percent_label(self.pct_24h);
        let width = ticker_tape_item_width(&self.ticker, &price_label, &percent_label);
        PreparedTickerTapeItem {
            symbol: self.symbol,
            ticker: self.ticker,
            price_label,
            percent_label,
            pct_24h: self.pct_24h,
            width,
        }
    }
}

fn ticker_tape_item_width(ticker: &str, price_label: &str, percent_label: &str) -> f32 {
    let text_chars =
        ticker.chars().count() + price_label.chars().count() + percent_label.chars().count();
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

pub(super) fn exchange_stat_usd_label(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(format_compact_usd)
        .unwrap_or_else(|| "-".to_string())
}

pub(super) fn pct_color(pct: Option<f64>, theme: &Theme) -> Color {
    match pct {
        Some(value) if value >= 0.0 => theme.palette().success,
        Some(_) => theme.palette().danger,
        None => theme.extended_palette().background.weak.text,
    }
}
