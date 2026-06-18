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

/// A canvas-drawable SVG handle for an asset logo.
///
/// `symbol_icon` returns a widget, which cannot be placed inside a `canvas`
/// overlay. This returns the same embedded logo data as an
/// [`iced::widget::svg::Handle`] so it can be painted with
/// [`canvas::Frame::draw_svg`]. No color filter is applied, so the original
/// logo colors are preserved.
pub fn symbol_svg_handle(symbol: &str) -> Option<iced::widget::svg::Handle> {
    let embedded_file = embedded_asset_svg(symbol)?;
    Some(iced::widget::svg::Handle::from_memory(embedded_file.data))
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
    match (hip3_dex(a_key), hip3_dex(b_key)) {
        (Some(a_dex), Some(b_dex)) => hip3_dex_rank(a_dex)
            .cmp(&hip3_dex_rank(b_dex))
            .then_with(|| a_key.cmp(b_key)),
        _ => a_key.cmp(b_key),
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
