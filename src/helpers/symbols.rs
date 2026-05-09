#[derive(rust_embed::RustEmbed)]
#[folder = "assets/"]
struct Assets;

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
    // Some symbols are like "xyz:AAPL" or "HYPE/USDC". Resolve the displayed
    // asset ticker before looking up the embedded SVG.
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

    let file = Assets::get(&path_lower)
        .or_else(|| Assets::get(&path_upper))
        .or_else(|| Assets::get(&path_orig));

    if let Some(embedded_file) = file {
        let svg_widget = iced::widget::svg(iced::widget::svg::Handle::from_memory(
            embedded_file.data.into_owned(),
        ))
        .width(iced::Length::Fixed(size as f32))
        .height(iced::Length::Fixed(size as f32))
        .style(
            move |_theme: &iced::Theme, _status| iced::widget::svg::Style { color: Some(color) },
        );
        Some(svg_widget)
    } else {
        None
    }
}
