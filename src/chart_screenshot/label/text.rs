use super::super::bitmap::{bitmap_max_chars, is_bitmap_glyph_supported};

// ---------------------------------------------------------------------------
// Label Text
// ---------------------------------------------------------------------------

pub(crate) fn ticker_label_text(
    symbol: &str,
    timeframe: &str,
    available_width: u32,
    scale: u32,
) -> String {
    let max_chars = bitmap_max_chars(available_width, scale);
    let symbol = sanitize_label_component(symbol);
    let timeframe = sanitize_label_component(timeframe);

    if symbol.is_empty() {
        return truncate_chars(&timeframe, max_chars);
    }
    if timeframe.is_empty() {
        return truncate_chars(&symbol, max_chars);
    }

    let full = format!("{symbol} {timeframe}");
    if full.chars().count() <= max_chars {
        return full;
    }

    let timeframe_len = timeframe.chars().count();
    if max_chars <= timeframe_len {
        return truncate_chars(&timeframe, max_chars);
    }

    let symbol_max = max_chars.saturating_sub(timeframe_len + 1);
    format!("{} {}", truncate_chars(&symbol, symbol_max), timeframe)
        .trim()
        .to_string()
}

fn sanitize_label_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            let upper = ch.to_ascii_uppercase();
            if is_bitmap_glyph_supported(upper) {
                upper
            } else if ch.is_whitespace() {
                ' '
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect::<String>()
}
