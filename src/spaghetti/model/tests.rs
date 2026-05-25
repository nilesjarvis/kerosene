use super::*;

fn test_series(symbol: &str, color: Color) -> Series {
    Series {
        symbol: symbol.to_string(),
        display: symbol.to_string(),
        candles: Vec::new(),
        color,
        loaded: false,
    }
}

#[test]
fn single_color_mode_recolors_all_series() {
    let theme = Theme::Dark;
    let mut canvas = SpaghettiCanvas::new();
    canvas
        .series
        .push(test_series("BTC", Color::from_rgb8(1, 2, 3)));
    canvas
        .series
        .push(test_series("ETH", Color::from_rgb8(4, 5, 6)));
    canvas.color_mode = ComparisonColorMode::Single;

    canvas.apply_style_colors(&theme);

    let expected = SpaghettiCanvas::single_color(&theme);
    assert!(canvas.series.iter().all(|series| series.color == expected));
    assert!(canvas.effective_show_labels());
}

#[test]
fn multi_color_mode_restores_theme_palette_series_colors() {
    let theme = Theme::Dark;
    let mut canvas = SpaghettiCanvas::new();
    canvas.series.push(test_series("BTC", Color::BLACK));
    canvas.series.push(test_series("ETH", Color::BLACK));
    canvas.color_mode = ComparisonColorMode::Multi;

    canvas.apply_style_colors(&theme);

    let colors = series_colors(&theme);
    assert_eq!(canvas.series[0].color, colors[0]);
    assert_eq!(canvas.series[1].color, colors[1]);
}
