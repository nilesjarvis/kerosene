use super::*;
use crate::api::Candle;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::renderer::Headless;
use iced::{Font, Pixels, Rectangle, Renderer};

const WIDTH: u32 = 500;
const HEIGHT: u32 = 300;
const CHART_W: f32 = 400.0;
const CHART_H: f32 = 280.0;

fn annotation_chart() -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(
        (1..=20)
            .map(|index| {
                Candle::test_price(index * 1_000, if index % 2 == 0 { 100.0 } else { 110.0 })
            })
            .collect(),
    );
    let start = (6_000, 102.0);
    let end = (15_000, 108.0);
    chart.annotations = [
        AnnotationKind::HorizontalLevel { price: 101.0 },
        AnnotationKind::TrendLine { start, end },
        AnnotationKind::Ray {
            start: (9_000, 103.0),
            end: (12_000, 105.0),
        },
        AnnotationKind::ExtendedLine {
            start: (10_000, 106.0),
            end: (14_000, 104.0),
        },
        AnnotationKind::Rectangle { a: start, b: end },
        AnnotationKind::Measure {
            start: (9_000, 103.0),
            end: (18_000, 106.0),
        },
        AnnotationKind::Fib {
            kind: FibKind::Retracement,
            points: vec![start, end],
        },
        AnnotationKind::Fib {
            kind: FibKind::Extension,
            points: vec![start, end, (18_000, 104.0)],
        },
        AnnotationKind::VerticalLine { time: 11_000 },
        AnnotationKind::TrendLine {
            start: (1_000, 108.0),
            end: (18_000, 102.0),
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(index, kind)| Annotation {
        id: index as u64 + 1,
        kind,
        style: AnnotationStyle {
            label: Some(format!("synthetic {index}")),
            locked: index == 4,
            visible: index != 9,
            ..Default::default()
        },
    })
    .collect();
    chart
}

fn render(
    chart: &CandlestickChart,
    state: &ChartState,
    renderer: &mut Renderer,
    theme: &Theme,
    fisheye: ChartFisheye,
) -> Vec<u8> {
    let bounds = Rectangle::with_size(Size::new(WIDTH as f32, HEIGHT as f32));
    let mut frame = canvas::Frame::new(renderer, bounds.size());
    let (hi, range, price_h) = chart
        .visible_price_params(state, CHART_W, CHART_H)
        .expect("price range");
    let price_to_y = |price| chart.price_to_y_with(price, hi, range, price_h);
    let badges =
        chart.right_axis_badge_layout(state, price_h, range, CHART_W, fisheye, &price_to_y);
    chart.draw_annotation_overlays(&mut AnnotationOverlayContext {
        frame: &mut frame,
        state,
        theme,
        chart_w: CHART_W,
        chart_h: CHART_H,
        price_h,
        price_range: range,
        right_axis_badges: &badges,
        fisheye,
        price_to_y: &price_to_y,
    });
    iced::advanced::Renderer::reset(renderer, bounds);
    renderer.draw_geometry(frame.into_geometry());
    let pixels = renderer.screenshot(Size::new(WIDTH, HEIGHT), 1.0, theme.palette().background);
    assert_eq!(pixels.len(), (WIDTH * HEIGHT * 4) as usize);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
    pixels
}

fn save_preview(name: &str, pixels: &[u8]) {
    if let Some(directory) = std::env::var_os("KEROSENE_ANNOTATION_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("preview directory");
        image::save_buffer(
            directory.join(format!("{name}.png")),
            pixels,
            WIDTH,
            HEIGHT,
            image::ColorType::Rgba8,
        )
        .expect("save synthetic annotation preview");
    }
}

#[tokio::test]
async fn selected_and_live_annotations_render_with_their_original_handles() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for theme in [Theme::Dark, Theme::Light] {
        for distorted in [false, true] {
            let fisheye = ChartFisheye::new(distorted, 0.7, CHART_W, CHART_H)
                .with_chromatic(distorted, 0.5)
                .with_edge_blur(distorted, 0.5);
            for selected in [2, 5, 7, 10] {
                let mut chart = annotation_chart();
                chart.active_tool = Some(DrawingTool::Select);
                let mut state = ChartState {
                    selected_annotation: Some(selected),
                    ..Default::default()
                };
                let before = render(&chart, &state, &mut renderer, &theme, fisheye);
                save_preview(&format!("{theme}-{distorted}-{selected}-selected"), &before);
                let mut live = chart
                    .annotations
                    .iter()
                    .find(|annotation| annotation.id == selected)
                    .expect("selected annotation")
                    .clone();
                live.kind.translate(1_000, 0.5);
                state.drag_annotation = Some(live);
                // Live handles remain visible even after leaving Select mode.
                chart.active_tool = None;
                let dragging = render(&chart, &state, &mut renderer, &theme, fisheye);
                save_preview(
                    &format!("{theme}-{distorted}-{selected}-dragging"),
                    &dragging,
                );
                assert!(before != dragging, "live geometry should move");
                state.drag_annotation = None;
                chart.active_tool = Some(DrawingTool::Select);
                assert!(
                    before == render(&chart, &state, &mut renderer, &theme, fisheye),
                    "preview must not mutate stored annotations"
                );
            }
        }
    }
}
