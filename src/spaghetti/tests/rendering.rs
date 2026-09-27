use super::*;
use crate::config::ChartCrosshairStyle;
use crate::spaghetti::SpaghettiChartState;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::renderer::Headless;
use iced::widget::canvas::Program;
use iced::{Font, Pixels, Point, Rectangle, Renderer, Size, Theme, mouse};

const START: u64 = 1_800_000_000_000;
const HOUR: u64 = 3_600_000;
const HEIGHT: u32 = 240;

fn comparison_chart() -> SpaghettiCanvas {
    let mut chart = SpaghettiCanvas::new();
    chart.series = [
        ("ALPHA", 100.0, Color::from_rgb(0.2, 0.7, 0.4)),
        ("BETA", 50.0, Color::from_rgb(0.8, 0.3, 0.2)),
    ]
    .into_iter()
    .map(|(symbol, price, color)| Series {
        symbol: symbol.into(),
        display: symbol.into(),
        color,
        loaded: true,
        candles: (0..=12)
            .map(|index| {
                candle_at(
                    START + index * HOUR,
                    price + (index as f64 * 0.8).sin() * 5.0,
                )
            })
            .collect(),
    })
    .collect();
    chart
}

fn render(
    chart: &SpaghettiCanvas,
    state: &SpaghettiChartState,
    renderer: &mut Renderer,
    theme: &Theme,
    width: u32,
) -> Vec<u8> {
    let bounds = Rectangle::with_size(Size::new(width as f32, HEIGHT as f32));
    let geometry = chart.draw(state, renderer, theme, bounds, mouse::Cursor::Unavailable);
    assert_eq!(geometry.len(), 2, "data and crosshair layers should render");
    iced::advanced::Renderer::reset(renderer, bounds);
    for layer in geometry {
        renderer.draw_geometry(layer);
    }
    let pixels = renderer.screenshot(Size::new(width, HEIGHT), 1.0, theme.palette().background);
    assert_eq!(pixels.len(), (width * HEIGHT * 4) as usize);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
    pixels
}

fn save_preview(name: &str, pixels: &[u8], width: u32) {
    // Synthetic previews support direct before/after image comparisons.
    if let Some(directory) = std::env::var_os("KEROSENE_COMPARISON_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("preview directory");
        image::save_buffer(
            directory.join(format!("{name}.png")),
            pixels,
            width,
            HEIGHT,
            image::ColorType::Rgba8,
        )
        .expect("save synthetic comparison preview");
    }
}

fn time_axis_pixels(pixels: &[u8], width: u32) -> Vec<u8> {
    let chart_w = width as usize - super::super::PRICE_AXIS_WIDTH as usize;
    pixels
        .chunks_exact((width * 4) as usize)
        .skip(HEIGHT as usize - super::super::TIME_AXIS_HEIGHT as usize + 2)
        .flat_map(|row| row[..chart_w * 4].iter().copied())
        .collect()
}

#[tokio::test]
async fn comparison_modes_render_with_the_same_time_axis_across_viewports() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for theme in [Theme::Dark, Theme::Light] {
        for width in [108, 432, 792] {
            let chart_w = width as f32 - super::super::PRICE_AXIS_WIDTH;
            for view in 0..3 {
                let mut chart = comparison_chart();
                let mut state = SpaghettiChartState {
                    px_per_ms: f64::from(chart_w) / (12 * HOUR) as f64,
                    cursor_position: Some(Point::new(chart_w * 0.5, 80.0)),
                    ..Default::default()
                };
                if view > 0 {
                    chart.base_timestamp = Some(START);
                    chart.active_session = Some(Session::UtcDay);
                    state.px_per_ms = DEFAULT_PX_PER_MS * if view == 1 { 0.5 } else { 2.0 };
                }
                if view == 2 {
                    state.scroll_offset_ms = (3 * HOUR) as f64;
                    state.y_auto = false;
                    state.y_scale = 2.0;
                }
                for (dotted, gradient) in
                    [(false, false), (true, false), (false, true), (true, true)]
                {
                    chart.dotted_background = dotted;
                    chart.gradient_background = gradient;
                    let mut time_axis = None;
                    for mode in 0..3 {
                        chart.pair_ratio_mode = mode > 0;
                        chart.pair_candle_mode = mode == 2;
                        let pixels = render(&chart, &state, &mut renderer, &theme, width);
                        let axis_pixels = time_axis_pixels(&pixels, width);
                        if let Some(expected) = &time_axis {
                            assert!(
                                axis_pixels == *expected,
                                "time axis changed with mode {mode}"
                            );
                        } else {
                            time_axis = Some(axis_pixels);
                        }
                        save_preview(
                            &format!("{theme}-{width}-{view}-{dotted}-{gradient}-{mode}"),
                            &pixels,
                            width,
                        );
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn comparison_crosshair_respects_plot_bounds() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let width = 432;
    let chart_w = width as f32 - super::super::PRICE_AXIS_WIDTH;
    let chart_h = HEIGHT as f32 - super::super::TIME_AXIS_HEIGHT;
    let mut chart = comparison_chart();
    let mut state = SpaghettiChartState {
        px_per_ms: 30.0 / HOUR as f64,
        ..Default::default()
    };
    for mode in 0..3 {
        chart.pair_ratio_mode = mode > 0;
        chart.pair_candle_mode = mode == 2;
        state.cursor_position = None;
        let unhovered = render(&chart, &state, &mut renderer, &Theme::Dark, width);
        for (index, (position, visible)) in [
            (Point::new(180.0, 80.0), true),
            (Point::ORIGIN, true),
            (Point::new(-1.0, 80.0), true),
            (Point::new(180.0, -1.0), true),
            (Point::new(chart_w, 80.0), false),
            (Point::new(180.0, chart_h), false),
            (Point::new(chart_w + 1.0, chart_h + 1.0), false),
            (Point::new(f32::NAN, 80.0), false),
            (Point::new(180.0, f32::NAN), false),
        ]
        .into_iter()
        .enumerate()
        {
            state.cursor_position = Some(position);
            let pixels = render(&chart, &state, &mut renderer, &Theme::Dark, width);
            assert_eq!(pixels != unhovered, visible, "mode {mode}, cursor {index}");
            save_preview(&format!("cursor-{mode}-{index}"), &pixels, width);
        }
    }
}

#[tokio::test]
async fn comparison_crosshair_styles_share_time_labels() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut chart = comparison_chart();
    let state = SpaghettiChartState {
        px_per_ms: 30.0 / HOUR as f64,
        cursor_position: Some(Point::new(180.0, 80.0)),
        ..Default::default()
    };
    for style in ChartCrosshairStyle::CROSSHAIRS
        .into_iter()
        .chain(ChartCrosshairStyle::GAME_HUDS)
        .chain([ChartCrosshairStyle::StackedRectangles])
    {
        chart.crosshair_style = style;
        for (guides, scale) in [(false, 0.75), (true, 1.5)] {
            chart.crosshair_guides_enabled = guides;
            chart.crosshair_scale = scale;
            let mut time_axis = None;
            for mode in 0..3 {
                chart.pair_ratio_mode = mode > 0;
                chart.pair_candle_mode = mode == 2;
                let pixels = render(&chart, &state, &mut renderer, &Theme::Dark, 432);
                let axis_pixels = time_axis_pixels(&pixels, 432);
                if let Some(expected) = &time_axis {
                    assert!(
                        axis_pixels == *expected,
                        "style {style:?}, guides {guides}, mode {mode}"
                    );
                } else {
                    time_axis = Some(axis_pixels);
                }
                save_preview(&format!("style-{style:?}-{guides}-{mode}"), &pixels, 432);
            }
        }
    }
}

#[tokio::test]
async fn pair_ratio_falls_back_to_normalized_with_one_loaded_series() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut chart = comparison_chart();
    chart.series[1].loaded = false;
    let state = SpaghettiChartState {
        px_per_ms: 30.0 / HOUR as f64,
        ..Default::default()
    };
    let normalized = render(&chart, &state, &mut renderer, &Theme::Dark, 432);
    chart.pair_ratio_mode = true;
    chart.pair_candle_mode = true;
    let fallback = render(&chart, &state, &mut renderer, &Theme::Dark, 432);
    assert!(normalized == fallback);
}
