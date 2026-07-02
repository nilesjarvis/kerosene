use std::cmp::Ordering;

#[derive(rust_embed::RustEmbed)]
#[folder = "assets/"]
struct Assets;

const HIP3_DEX_ORDER: &[&str] = &["xyz", "flx", "vntl", "hyna", "km", "abcd", "cash", "para"];

pub fn category_color(category: &str, theme: &iced::Theme) -> iced::Color {
    let palette = theme.palette();
    let ext_palette = theme.extended_palette();
    match category {
        "crypto" => palette.success,
        "stocks" => ext_palette.primary.base.color,
        "commodities" => palette.warning,
        "indices" => ext_palette.secondary.base.color,
        "fx" => ext_palette.primary.strong.color,
        "preipo" => ext_palette.danger.weak.color,
        "spot" => ext_palette.success.strong.color,
        "outcome" => ext_palette.primary.weak.color,
        _ => ext_palette.background.neutral.color,
    }
}

pub fn symbol_icon<'a>(
    symbol: &str,
    size: u16,
    color: iced::Color,
) -> Option<iced::widget::Svg<'a, iced::Theme>> {
    let embedded_file = embedded_asset_svg(symbol)?;

    let svg_widget = iced::widget::svg(iced::widget::svg::Handle::from_memory(embedded_file.data))
        .width(iced::Length::Fixed(size as f32))
        .height(iced::Length::Fixed(size as f32))
        .style(
            move |_theme: &iced::Theme, _status| iced::widget::svg::Style { color: Some(color) },
        );
    Some(svg_widget)
}

/// A canvas-drawable asset logo: its SVG [`Handle`](iced::widget::svg::Handle)
/// paired with the logo's intrinsic aspect ratio (`width / height`).
///
/// `symbol_icon` returns a widget, which cannot be placed inside a `canvas`
/// overlay. This returns the embedded logo data as a handle for
/// [`canvas::Frame::draw_svg`], plus the aspect ratio so the caller can size
/// the draw rectangle to the logo and avoid distorting or clipping non-square
/// logos (the rasterizer scales to fit while preserving aspect, so a square
/// target letterboxes tall logos and clips wide ones). Falls back to a square
/// (`1.0`) aspect when the SVG declares no parseable `viewBox` or size.
pub fn symbol_svg_logo(symbol: &str) -> Option<(iced::widget::svg::Handle, f32)> {
    let embedded_file = embedded_asset_svg(symbol)?;
    let aspect = svg_aspect_ratio(&embedded_file.data).unwrap_or(1.0);
    Some((
        iced::widget::svg::Handle::from_memory(embedded_file.data),
        aspect,
    ))
}

/// Intrinsic aspect ratio (`width / height`) of an SVG document, read from the
/// opening `<svg>` tag. Prefers the `viewBox` (`min-x min-y width height`) and
/// falls back to the `width`/`height` attributes. Returns `None` if neither
/// yields two positive dimensions.
fn svg_aspect_ratio(bytes: &[u8]) -> Option<f32> {
    let text = std::str::from_utf8(bytes).ok()?;
    let start = text.find("<svg")?;
    let tag_end = text[start..].find('>')? + start;
    let tag = &text[start..tag_end];

    if let Some(view_box) = tag_attr(tag, "viewBox") {
        let nums: Vec<f32> = view_box
            .split([' ', ',', '\t', '\n', '\r'])
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<f32>().ok())
            .collect();
        if let [_, _, w, h] = nums.as_slice()
            && *w > 0.0
            && *h > 0.0
        {
            return Some(w / h);
        }
    }

    match (
        tag_attr(tag, "width").and_then(parse_svg_len),
        tag_attr(tag, "height").and_then(parse_svg_len),
    ) {
        (Some(w), Some(h)) if w > 0.0 && h > 0.0 => Some(w / h),
        _ => None,
    }
}

/// Reads the value of attribute `attr` from an opening tag, requiring the name
/// to begin at a whitespace boundary so `width` does not match `stroke-width`.
fn tag_attr<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
    let bytes = tag.as_bytes();
    let mut from = 0;
    while let Some(rel) = tag[from..].find(attr) {
        let start = from + rel;
        let after = start + attr.len();
        let at_boundary = start == 0 || bytes[start - 1].is_ascii_whitespace();
        if at_boundary && let Some(rest) = tag[after..].trim_start().strip_prefix('=') {
            let mut chars = rest.trim_start().chars();
            if let Some(quote @ ('"' | '\'')) = chars.next() {
                let inner = chars.as_str();
                if let Some(end) = inner.find(quote) {
                    return Some(&inner[..end]);
                }
            }
        }
        from = after;
    }
    None
}

/// Parses the leading number of an SVG length, ignoring any unit suffix
/// (`px`, `%`, `em`, …).
fn parse_svg_len(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    let end = trimmed
        .find(|c: char| !c.is_ascii_digit() && !matches!(c, '.' | '-' | '+'))
        .unwrap_or(trimmed.len());
    trimmed[..end].parse::<f32>().ok()
}

/// Resolves the embedded SVG logo for a symbol, trying the displayed asset
/// ticker in lower, upper, then original case. For qualified symbols like
/// `xyz:AAPL` the dex prefix is stripped (last `:` segment); for pairs like
/// `HYPE/USDC` or hyphenated like `kPEPE-USDC` the base asset is the first
/// segment.
fn embedded_asset_svg(symbol: &str) -> Option<rust_embed::EmbeddedFile> {
    let base_name = symbol
        .split(':')
        .next_back()?
        .split('/')
        .next()?
        .split('-')
        .next()?;
    let path_lower = format!("{}.svg", base_name.to_lowercase());
    let path_upper = format!("{}.svg", base_name.to_uppercase());
    let path_orig = format!("{base_name}.svg");

    Assets::get(&path_lower)
        .or_else(|| Assets::get(&path_upper))
        .or_else(|| Assets::get(&path_orig))
}

pub fn hip3_dex(symbol: &str) -> Option<&str> {
    let (dex, asset) = symbol.split_once(':')?;
    (!dex.is_empty() && !asset.is_empty()).then_some(dex)
}

pub fn compare_symbol_keys_for_same_ticker(a_key: &str, b_key: &str) -> Ordering {
    market_rank_for_symbol_key(a_key)
        .cmp(&market_rank_for_symbol_key(b_key))
        .then_with(|| match (hip3_dex(a_key), hip3_dex(b_key)) {
            (Some(a_dex), Some(b_dex)) => hip3_dex_rank(a_dex)
                .cmp(&hip3_dex_rank(b_dex))
                .then_with(|| a_key.cmp(b_key)),
            _ => a_key.cmp(b_key),
        })
}

/// Same-ticker collisions rank perps first so list ordering agrees with the
/// bare-ticker resolver (`resolve_exchange_symbol_by_key_or_ticker`) and
/// `switch_active_symbol_internal`, both of which prefer the perp market.
/// Perp keys are plain names (optionally `dex:`-prefixed); spot pair keys
/// start with `@` or are API-named pairs like "PURR/USDC", and outcome keys
/// start with `#`.
fn market_rank_for_symbol_key(key: &str) -> u8 {
    match key.as_bytes().first() {
        Some(b'@') => 1,
        Some(b'#') => 2,
        _ if key.contains('/') => 1,
        _ => 0,
    }
}

fn hip3_dex_rank(dex: &str) -> (usize, &str) {
    (
        HIP3_DEX_ORDER
            .iter()
            .position(|known| *known == dex)
            .unwrap_or(usize::MAX),
        dex,
    )
}

#[cfg(test)]
mod tests;
