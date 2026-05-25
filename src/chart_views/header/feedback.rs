use crate::chart_state::{CHART_PRICE_FLASH_MS, PriceFlash, PriceFlashDirection};
use crate::helpers::{format_decimal_with_commas, invalid_data_placeholder};

use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Header Feedback
// ---------------------------------------------------------------------------

pub(super) fn format_signed_usd_change(value: f64) -> String {
    if !value.is_finite() {
        return invalid_data_placeholder();
    }
    let sign = if value.is_sign_negative() { "-" } else { "+" };
    format!("{sign}${}", format_decimal_with_commas(value.abs(), 2))
}

pub(super) fn chart_header_price_flash_color(
    flash: Option<PriceFlash>,
    now_ms: u64,
    theme: &Theme,
) -> Option<Color> {
    let flash = flash?;
    let elapsed = now_ms.saturating_sub(flash.started_at_ms);
    if elapsed >= CHART_PRICE_FLASH_MS {
        return None;
    }

    let base = match flash.direction {
        PriceFlashDirection::Up => theme.palette().success,
        PriceFlashDirection::Down => theme.palette().danger,
    };
    let target = theme.palette().text;
    let factor = (elapsed as f32 / CHART_PRICE_FLASH_MS as f32).clamp(0.0, 1.0);

    Some(Color::from_rgba(
        base.r + (target.r - base.r) * factor,
        base.g + (target.g - base.g) * factor,
        base.b + (target.b - base.b) * factor,
        1.0,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChartHeaderChangedText {
    pub(super) before: String,
    pub(super) changed: String,
    pub(super) after: String,
}

pub(super) fn chart_header_changed_text(
    previous: &str,
    current: &str,
) -> Option<ChartHeaderChangedText> {
    if previous == current {
        return None;
    }

    let previous_chars = previous.chars().collect::<Vec<_>>();
    let current_chars = current.chars().collect::<Vec<_>>();

    let prefix_len = previous_chars
        .iter()
        .zip(current_chars.iter())
        .take_while(|(previous, current)| previous == current)
        .count();

    let max_suffix_len = previous_chars
        .len()
        .min(current_chars.len())
        .saturating_sub(prefix_len);
    let suffix_len = previous_chars
        .iter()
        .rev()
        .zip(current_chars.iter().rev())
        .take(max_suffix_len)
        .take_while(|(previous, current)| previous == current)
        .count();

    let changed_end = current_chars.len().saturating_sub(suffix_len);
    let changed = current_chars[prefix_len..changed_end]
        .iter()
        .collect::<String>();
    if changed.is_empty() {
        return None;
    }

    Some(ChartHeaderChangedText {
        before: current_chars[..prefix_len].iter().collect(),
        changed,
        after: current_chars[changed_end..].iter().collect(),
    })
}
